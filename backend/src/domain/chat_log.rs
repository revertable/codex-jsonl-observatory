use indexmap::IndexMap;

use super::{ChatEntryFilter, RenderedEntry, SessionProvenance, TranscriptBlock};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedChatLog {
    pub entries: Vec<RenderedEntry>,
    pub entry_timestamps: Vec<Option<String>>,
    pub session_provenance: SessionProvenance,
    pub parsed_candidates: usize,
    pub ignored_lines: usize,
    pub malformed_lines: usize,
    pub observed_event_counts: IndexMap<String, usize>,
}

impl ParsedChatLog {
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
            entry_timestamps: Vec::new(),
            session_provenance: SessionProvenance::default(),
            parsed_candidates: 0,
            ignored_lines: 0,
            malformed_lines: 0,
            observed_event_counts: IndexMap::new(),
        }
    }

    pub fn filtered(&self, filter: &ChatEntryFilter) -> Self {
        Self {
            entries: self
                .entries
                .iter()
                .filter(|entry| filter.allows(entry.kind))
                .cloned()
                .collect(),
            entry_timestamps: self
                .entries
                .iter()
                .zip(&self.entry_timestamps)
                .filter(|(entry, _)| filter.allows(entry.kind))
                .map(|(_, timestamp)| timestamp.clone())
                .collect(),
            session_provenance: self.session_provenance.clone(),
            parsed_candidates: self.parsed_candidates,
            ignored_lines: self.ignored_lines,
            malformed_lines: self.malformed_lines,
            observed_event_counts: self.observed_event_counts.clone(),
        }
    }

    pub fn transcript_blocks(&self) -> Vec<TranscriptBlock> {
        self.entries
            .iter()
            .map(|entry| {
                let label = entry.kind.label();
                TranscriptBlock {
                    entry_type: entry.kind,
                    label,
                    title: label,
                    content: entry.content.clone(),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ReferencedConversation, RenderedEntryKind};

    fn sample_log() -> ParsedChatLog {
        let mut observed_event_counts = IndexMap::new();
        observed_event_counts.insert("event_msg/user_message".to_owned(), 2);
        observed_event_counts.insert("response_item/message".to_owned(), 1);

        ParsedChatLog {
            entries: vec![
                entry(RenderedEntryKind::Context, "context"),
                entry(RenderedEntryKind::Task, "task"),
                entry(RenderedEntryKind::You, "you"),
                entry(RenderedEntryKind::Codex, "codex"),
                entry(RenderedEntryKind::ToolCall, "tool call"),
                entry(RenderedEntryKind::ToolResult, "tool result"),
                entry(RenderedEntryKind::System, "system"),
            ],
            entry_timestamps: vec![None; 7],
            session_provenance: SessionProvenance::default(),
            parsed_candidates: 9,
            ignored_lines: 3,
            malformed_lines: 2,
            observed_event_counts,
        }
    }

    fn entry(kind: RenderedEntryKind, content: &str) -> RenderedEntry {
        RenderedEntry {
            kind,
            content: content.to_owned(),
        }
    }

    #[test]
    fn parsed_entries_can_be_filtered_without_reparsing() {
        let parsed = sample_log();
        let filtered = parsed.filtered(&ChatEntryFilter {
            show_you: false,
            ..ChatEntryFilter::all()
        });

        assert_eq!(filtered.parsed_candidates, parsed.parsed_candidates);
        assert_eq!(filtered.ignored_lines, parsed.ignored_lines);
        assert_eq!(filtered.malformed_lines, parsed.malformed_lines);
        assert_eq!(filtered.observed_event_counts, parsed.observed_event_counts);
        assert_eq!(parsed.entries.len(), 7);
        assert_eq!(filtered.entries.len(), 6);
    }

    #[test]
    fn filtering_preserves_entry_order() {
        let filtered = sample_log().filtered(&ChatEntryFilter {
            show_you: false,
            show_tool_call: false,
            ..ChatEntryFilter::all()
        });

        assert_eq!(
            filtered.entries,
            vec![
                entry(RenderedEntryKind::Context, "context"),
                entry(RenderedEntryKind::Task, "task"),
                entry(RenderedEntryKind::Codex, "codex"),
                entry(RenderedEntryKind::ToolResult, "tool result"),
                entry(RenderedEntryKind::System, "system"),
            ]
        );
    }

    #[test]
    fn turning_off_you_hides_only_you_entries() {
        let filtered = sample_log().filtered(&ChatEntryFilter {
            show_you: false,
            ..ChatEntryFilter::all()
        });

        assert!(
            !filtered
                .entries
                .iter()
                .any(|entry| entry.kind == RenderedEntryKind::You)
        );
        assert!(
            filtered
                .entries
                .iter()
                .any(|entry| entry.kind == RenderedEntryKind::Task)
        );
    }

    #[test]
    fn turning_off_codex_hides_codex_entries() {
        let filtered = sample_log().filtered(&ChatEntryFilter {
            show_codex: false,
            ..ChatEntryFilter::all()
        });

        assert!(
            !filtered
                .entries
                .iter()
                .any(|entry| entry.kind == RenderedEntryKind::Codex)
        );
    }

    #[test]
    fn turning_off_tool_call_hides_tool_call_entries() {
        let filtered = sample_log().filtered(&ChatEntryFilter {
            show_tool_call: false,
            ..ChatEntryFilter::all()
        });

        assert!(
            !filtered
                .entries
                .iter()
                .any(|entry| entry.kind == RenderedEntryKind::ToolCall)
        );
    }

    #[test]
    fn turning_off_tool_result_hides_tool_result_entries() {
        let filtered = sample_log().filtered(&ChatEntryFilter {
            show_tool_result: false,
            ..ChatEntryFilter::all()
        });

        assert!(
            !filtered
                .entries
                .iter()
                .any(|entry| entry.kind == RenderedEntryKind::ToolResult)
        );
    }

    #[test]
    fn turning_off_meta_hides_context_task_and_system_entries() {
        let filtered = sample_log().filtered(&ChatEntryFilter {
            show_meta: false,
            ..ChatEntryFilter::all()
        });

        assert!(!filtered.entries.iter().any(|entry| matches!(
            entry.kind,
            RenderedEntryKind::Context | RenderedEntryKind::Task | RenderedEntryKind::System
        )));
    }

    #[test]
    fn transcript_blocks_preserve_entry_type_label_title_and_content() {
        let parsed = ParsedChatLog {
            entries: vec![
                entry(RenderedEntryKind::You, "hello"),
                entry(RenderedEntryKind::Codex, "hi"),
            ],
            entry_timestamps: vec![None; 2],
            session_provenance: SessionProvenance::default(),
            parsed_candidates: 2,
            ignored_lines: 0,
            malformed_lines: 0,
            observed_event_counts: IndexMap::new(),
        };

        assert_eq!(
            parsed.transcript_blocks(),
            vec![
                TranscriptBlock {
                    entry_type: RenderedEntryKind::You,
                    label: "[YOU]",
                    title: "[YOU]",
                    content: "hello".to_owned(),
                },
                TranscriptBlock {
                    entry_type: RenderedEntryKind::Codex,
                    label: "[CODEX]",
                    title: "[CODEX]",
                    content: "hi".to_owned(),
                }
            ]
        );
    }

    #[test]
    fn filtering_preserves_session_provenance_independently_from_entries() {
        let reference = ReferencedConversation {
            conversation_id: Some("conversation-1".to_owned()),
            title: Some("Reference".to_owned()),
            preview_available: true,
        };
        let parsed = ParsedChatLog {
            entries: vec![
                entry(RenderedEntryKind::Codex, "before"),
                entry(RenderedEntryKind::You, "continue"),
            ],
            entry_timestamps: vec![None; 2],
            session_provenance: SessionProvenance {
                referenced_conversations: vec![reference.clone()],
            },
            parsed_candidates: 2,
            ignored_lines: 0,
            malformed_lines: 0,
            observed_event_counts: IndexMap::new(),
        };

        let filtered = parsed.filtered(&ChatEntryFilter {
            show_codex: false,
            ..ChatEntryFilter::all()
        });
        let blocks = filtered.transcript_blocks();

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].content, "continue");
        assert_eq!(
            filtered.session_provenance.referenced_conversations,
            vec![reference]
        );
    }
}

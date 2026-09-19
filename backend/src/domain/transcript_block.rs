use super::RenderedEntryKind;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranscriptBlock {
    pub entry_type: RenderedEntryKind,
    pub label: &'static str,
    pub title: &'static str,
    pub timestamp: Option<String>,
    pub content: String,
}

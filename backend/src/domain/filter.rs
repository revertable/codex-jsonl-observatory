use super::RenderedEntryKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChatEntryFilter {
    pub show_you: bool,
    pub show_codex: bool,
    pub show_tool_call: bool,
    pub show_tool_result: bool,
    pub show_meta: bool,
}

impl ChatEntryFilter {
    pub const fn all() -> Self {
        Self {
            show_you: true,
            show_codex: true,
            show_tool_call: true,
            show_tool_result: true,
            show_meta: true,
        }
    }

    pub const fn allows(&self, kind: RenderedEntryKind) -> bool {
        match kind {
            RenderedEntryKind::Context | RenderedEntryKind::Task | RenderedEntryKind::System => {
                self.show_meta
            }
            RenderedEntryKind::You => self.show_you,
            RenderedEntryKind::Codex => self.show_codex,
            RenderedEntryKind::ToolCall => self.show_tool_call,
            RenderedEntryKind::ToolResult => self.show_tool_result,
        }
    }
}

impl Default for ChatEntryFilter {
    fn default() -> Self {
        Self::all()
    }
}

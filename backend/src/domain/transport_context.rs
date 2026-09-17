#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SessionProvenance {
    pub referenced_conversations: Vec<ReferencedConversation>,
}

impl SessionProvenance {
    pub fn observe_reference(&mut self, reference: ReferencedConversation) {
        if let Some(existing) = self.referenced_conversations.iter_mut().find(|existing| {
            existing.conversation_id == reference.conversation_id
                && existing.title == reference.title
        }) {
            existing.preview_available |= reference.preview_available;
        } else {
            self.referenced_conversations.push(reference);
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferencedConversation {
    pub conversation_id: Option<String>,
    pub title: Option<String>,
    pub preview_available: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_reference_merges_preview_availability_by_identity() {
        let mut provenance = SessionProvenance::default();
        provenance.observe_reference(ReferencedConversation {
            conversation_id: Some("conversation-1".to_owned()),
            title: Some("Reference".to_owned()),
            preview_available: false,
        });
        provenance.observe_reference(ReferencedConversation {
            conversation_id: Some("conversation-1".to_owned()),
            title: Some("Reference".to_owned()),
            preview_available: true,
        });

        assert_eq!(provenance.referenced_conversations.len(), 1);
        assert!(provenance.referenced_conversations[0].preview_available);
    }
}

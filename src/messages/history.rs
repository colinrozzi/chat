use super::{Message, StoredMessage};
use super::store::MessageStore;
use crate::bindings::ntwk::theater::runtime::log;

pub struct MessageHistory {
    store: MessageStore,
}

impl MessageHistory {
    pub fn new(store: MessageStore) -> Self {
        Self { store }
    }

    pub fn get_full_message_tree(&self, head: Option<String>) -> Result<Vec<StoredMessage>, Box<dyn std::error::Error>> {
        let mut messages = Vec::new();
        let mut current_id = head;

        while let Some(id) = current_id {
            let stored_msg = self.store.load_message(&id)?;
            current_id = stored_msg.parent();
            messages.push(stored_msg);
        }

        // Return messages in reverse order (oldest first)
        messages.reverse();
        Ok(messages)
    }

    pub fn get_child_responses(&self, message_id: &str) -> Result<Vec<StoredMessage>, Box<dyn std::error::Error>> {
        if let StoredMessage::Rollup(rollup) = self.store.load_message(message_id)? {
            let mut responses = Vec::new();
            for child_response in rollup.child_responses {
                if let Ok(msg) = self.store.load_message(&child_response.message_id) {
                    responses.push(msg);
                }
            }
            Ok(responses)
        } else {
            Ok(vec![])
        }
    }

    pub fn get_message_history(&self, head: Option<String>) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
        log("Getting message history");
        let mut messages = Vec::new();
        let mut stored_messages = Vec::new();
        let mut current_id = head;

        // First, collect all messages in the chain
        while let Some(id) = current_id {
            let stored_msg = self.store.load_message(&id)?;
            current_id = stored_msg.parent();
            stored_messages.push(stored_msg);
        }

        // Process stored messages in reverse (oldest first)
        for stored_msg in stored_messages.iter().rev() {
            match stored_msg {
                StoredMessage::Message(msg) => {
                    messages.push(msg.clone());
                }
                StoredMessage::Rollup(rollup) => {
                    // For each child response, load it and append to the previous message
                    if let Some(last_message) = messages.last_mut() {
                        let mut child_content = String::new();
                        for child_response in &rollup.child_responses {
                            if let Ok(child_msg) = self.store.load_message(&child_response.message_id) {
                                match child_msg {
                                    StoredMessage::Message(msg) => {
                                        child_content.push_str(&format!(
                                            "\nActor {} response:\n{}",
                                            child_response.child_id, msg.content
                                        ));
                                    }
                                    _ => continue,
                                }
                            }
                        }

                        // If we have child responses, append them to the last message
                        if !child_content.is_empty() {
                            last_message.content = format!(
                                "{}\n\nActor Responses:{}",
                                last_message.content, child_content
                            );
                        }
                    }
                }
            }
        }

        Ok(messages)
    }
}
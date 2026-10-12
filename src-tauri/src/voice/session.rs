//! What every speech engine looks like to a recording: audio goes in as it is heard, and
//! words come out, some to type now and some still being decided. The local engine and each
//! cloud provider are one of these.
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The engine is loaded or connected and hearing.
    Ready,
    /// Words that are settled: they are typed, and never revised. `boundary` is true when
    /// they begin a new phrase (cloud providers), false when they may go on the last word
    /// (the local engine sends pieces as small as part of a word).
    Committed { text: String, boundary: bool },
    /// The words after the settled ones, still being decided: shown, not typed. Replaces
    /// what was shown before.
    Tentative(String),
    /// Everything said has been settled after `finish`.
    Final,
    /// What went wrong, in words for the user. The recording ends.
    Fault(String),
}

pub trait Session: Send {
    /// Audio heard since the last call: 16 kHz mono in [-1, 1]. Queued; never blocks.
    fn feed(&mut self, pcm: &[f32]);
    /// A pause ended a phrase. Engines that settle words themselves ignore it.
    fn pause(&mut self) {}
    /// No more audio: settle everything. `Event::Final` follows.
    fn finish(&mut self);
    /// Discard everything now. Dropping a session does the same.
    fn cancel(&mut self);
    fn events(&self) -> &Receiver<Event>;
    /// How long the last words may take to settle after `finish`, before the recording
    /// ends with what it has.
    fn settle_within(&self) -> std::time::Duration {
        std::time::Duration::from_secs(4)
    }
}

/// Puts typed pieces together with the right spacing. The terminal write drops leading and
/// trailing spaces, so whether a piece needs a space before it is decided here.
#[derive(Debug, Default)]
pub struct Joiner {
    typed: bool,
    /// The last piece ended with a space, or a space-only piece came after it.
    space: bool,
}

impl Joiner {
    /// The piece to type and whether a space goes before it; None when there is nothing to type.
    pub fn next(&mut self, text: &str, boundary: bool) -> Option<(String, bool)> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            if !text.is_empty() {
                self.space = true;
            }
            return None;
        }
        let leading = text.starts_with(char::is_whitespace);
        let punctuation = trimmed.starts_with([',', '.', '?', '!', ':', ';', ')', ']', '\'', '%']);
        let space = self.typed && !punctuation && (boundary || leading || self.space);
        self.space = text.ends_with(char::is_whitespace);
        Some((trimmed.to_string(), space))
    }

    pub fn typed(&mut self) {
        self.typed = true;
    }

    pub fn has_typed(&self) -> bool {
        self.typed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn join(pieces: &[(&str, bool)]) -> String {
        let mut joiner = Joiner::default();
        let mut out = String::new();
        for (text, boundary) in pieces {
            if let Some((piece, space)) = joiner.next(text, *boundary) {
                if space {
                    out.push(' ');
                }
                out.push_str(&piece);
                joiner.typed();
            }
        }
        out
    }

    #[test]
    fn local_pieces_keep_the_words_they_split() {
        // The local engine settles text as it goes, sometimes mid-word.
        assert_eq!(join(&[("And so, my", false), (" fellow Amer", false), ("icans", false), (", ask not", false)]), "And so, my fellow Americans, ask not");
        // A space-only piece is remembered for the next word.
        assert_eq!(join(&[("fix the", false), (" ", false), ("test", false)]), "fix the test");
        assert_eq!(join(&[("fix the ", false), ("test.", false)]), "fix the test.");
    }

    #[test]
    fn cloud_phrases_are_parted_by_a_space_but_not_before_punctuation() {
        assert_eq!(join(&[("Refactor the checkout.", true), ("Then add a test", true), (".", true)]), "Refactor the checkout. Then add a test.");
        assert_eq!(join(&[("", true), ("   ", true), ("First", true)]), "First");
    }
}

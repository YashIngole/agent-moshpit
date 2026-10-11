//! Dictation as it is spoken. A phrase is settled at the pause after it and typed straight
//! away; the words since the last pause are previewed until the next one. Stopping leaves
//! only that last phrase to recognise. No model here: this decides what to recognise when.
use super::audio::RATE;

/// 20 ms of audio, the step levels are measured in.
const WINDOW: usize = RATE / 50;
/// How long a quiet stretch must last to end a phrase: 500 ms.
const PAUSE: usize = 25;
/// The shortest phrase settled on its own: shorter ones wait for the next.
const PHRASE: usize = RATE;
/// A phrase that runs this long without a pause is settled at its quietest moment,
/// so what is left to recognise after stopping stays short.
const LONGEST: usize = 20 * RATE;

/// What to recognise next, as a stretch of the recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Job {
    /// A finished phrase: its words are typed and never revised.
    Settle(usize, usize),
    /// The phrase still being spoken, shown but not typed.
    Preview(usize, usize),
}

/// The recording's loudness every 20 ms, judged against its own range: its quietest tenth
/// (the room) and its loudest tenth (speech). So a quiet room, a loud one and a quiet
/// microphone are each judged by their own levels.
pub struct Levels {
    levels: Vec<f64>,
    range: Option<(f64, f64)>,
}

impl Levels {
    pub fn of(pcm: &[f32]) -> Self {
        let levels: Vec<f64> = pcm
            .as_chunks::<WINDOW>()
            .0
            .iter()
            .map(|chunk| (chunk.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / WINDOW as f64).sqrt())
            .collect();
        // Under a second there is too little to tell speech from the room.
        let range = (levels.len() >= 50).then(|| {
            let mut sorted = levels.clone();
            sorted.sort_by(f64::total_cmp);
            (sorted[sorted.len() / 10], sorted[sorted.len() * 9 / 10])
        });
        Self { levels, range }
    }

    /// The level `share` of the way from the room to speech.
    fn above_room(&self, share: f64) -> Option<f64> {
        self.range.map(|(room, speech)| room + (speech - room) * share)
    }

    /// The middle of the first pause after `from` that ends a phrase of at least a second.
    /// Quiet, for a pause, is under a fifth of the way from the room to speech.
    pub fn pause(&self, from: usize) -> Option<usize> {
        let quiet = self.above_room(0.2)?;
        let (mut run, mut spoken) = (0, false);
        for (i, level) in self.levels.iter().enumerate().skip(from.div_ceil(WINDOW)) {
            if *level >= quiet {
                (run, spoken) = (0, true);
                continue;
            }
            run += 1;
            if run >= PAUSE && spoken {
                let cut = (i + 1 - PAUSE / 2) * WINDOW;
                if cut >= from + PHRASE {
                    return Some(cut);
                }
            }
        }
        None
    }

    /// The quietest 20 ms between `from` and `to`, to end a phrase that has no pause.
    pub fn quietest(&self, from: usize, to: usize) -> usize {
        let (first, last) = (from.div_ceil(WINDOW), (to / WINDOW).min(self.levels.len()));
        (first..last)
            .min_by(|a, b| self.levels[*a].total_cmp(&self.levels[*b]))
            .map_or(to, |i| i * WINDOW)
    }

    /// Whether everything from `from` on is as quiet as the room, so recognising it would add
    /// no words. Stricter than a pause: a soft last word must not be dropped.
    pub fn silent_from(&self, from: usize) -> bool {
        self.above_room(0.1).is_some_and(|quiet| self.levels.iter().skip(from / WINDOW).all(|level| *level < quiet))
    }
}

/// The next stretch to recognise, given what is settled and what the last preview covered.
/// Settling comes first: typed words are what the user is waiting for.
pub fn next(pcm: &[f32], settled: usize, previewed: usize) -> Option<Job> {
    let levels = Levels::of(pcm);
    if let Some(cut) = levels.pause(settled) {
        return Some(Job::Settle(settled, cut));
    }
    if pcm.len() - settled > LONGEST {
        let cut = levels.quietest(settled + LONGEST * 3 / 4, pcm.len());
        return Some(Job::Settle(settled, cut));
    }
    // A preview when there is half a second more to show, and something said since the pause.
    (pcm.len() >= settled + RATE / 2 && pcm.len() >= previewed + RATE / 2 && !levels.silent_from(settled))
        .then_some(Job::Preview(settled, pcm.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Speech-like sound: loud and varying, unlike the room around it.
    fn said(seconds: f32) -> Vec<f32> {
        (0..(seconds * RATE as f32) as usize)
            .map(|i| 0.3 * (i as f32 * 0.07).sin() * (0.7 + 0.3 * (i as f32 * 0.0008).sin()))
            .collect()
    }
    fn room(seconds: f32) -> Vec<f32> {
        let mut x = 3u32;
        (0..(seconds * RATE as f32) as usize)
            .map(|_| {
                x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((x >> 8) as f32 / (1 << 23) as f32 - 1.0) * 0.003
            })
            .collect()
    }
    fn joined(parts: &[Vec<f32>]) -> Vec<f32> {
        parts.concat()
    }

    #[test]
    fn a_phrase_settles_at_the_pause_after_it() {
        let pcm = joined(&[room(0.3), said(2.0), room(0.7), said(1.0)]);
        let Some(Job::Settle(0, cut)) = next(&pcm, 0, 0) else { panic!("{:?}", next(&pcm, 0, 0)) };
        // In the pause: after the words, before the next ones.
        assert!(cut > (2.3 * RATE as f32) as usize && cut < (3.0 * RATE as f32) as usize, "{cut}");
        // Once settled there, the next phrase is still being spoken: it is previewed.
        assert_eq!(next(&pcm, cut, 0), Some(Job::Preview(cut, pcm.len())));
    }

    #[test]
    fn short_breaths_inside_a_phrase_do_not_split_it() {
        let pcm = joined(&[said(1.5), room(0.3), said(1.5), room(0.2), said(1.0)]);
        assert_eq!(next(&pcm, 0, 0), Some(Job::Preview(0, pcm.len())));
    }

    #[test]
    fn a_word_too_short_to_stand_alone_waits_for_the_next() {
        let pcm = joined(&[said(0.4), room(0.8), said(1.5)]);
        assert!(!matches!(next(&pcm, 0, 0), Some(Job::Settle(..))));
    }

    #[test]
    fn previews_wait_for_more_audio_and_for_words() {
        let pcm = joined(&[room(0.5), said(1.5)]);
        assert_eq!(next(&pcm, 0, pcm.len() - RATE / 4), None);
        assert_eq!(next(&pcm, 0, 0), Some(Job::Preview(0, pcm.len())));
        // Nothing said since the last settled phrase: nothing to show.
        let pcm = joined(&[said(2.0), room(1.5)]);
        let Some(Job::Settle(0, cut)) = next(&pcm, 0, 0) else { panic!() };
        assert_eq!(next(&pcm, cut, 0), None);
        assert!(Levels::of(&pcm).silent_from(cut));
    }

    #[test]
    fn a_long_run_without_a_pause_is_settled_anyway() {
        let pcm = said(25.0);
        let Some(Job::Settle(0, cut)) = next(&pcm, 0, 0) else { panic!() };
        assert!(cut >= 15 * RATE && cut <= pcm.len(), "{cut}");
    }

    #[test]
    fn under_a_second_nothing_is_judged() {
        assert!(Levels::of(&said(0.5)).pause(0).is_none());
        assert!(!Levels::of(&room(0.5)).silent_from(0));
    }
}

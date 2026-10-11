//! Where a phrase ends, as it is spoken. Engines that settle words only when told (OpenAI,
//! ElevenLabs) are told at each pause, so words are typed while the user goes on talking.
//! Loudness is judged against the recording's own range: its quietest tenth (the room) and
//! its loudest tenth (speech), so a quiet room, a loud one and a quiet microphone are each
//! judged by their own levels.
use super::audio::RATE;

/// 20 ms of audio, the step levels are measured in.
const WINDOW: usize = RATE / 50;
/// How long a quiet stretch must last to end a phrase: 500 ms.
const PAUSE: usize = 25;
/// The shortest phrase ended on its own: a shorter one waits for the next pause.
const PHRASE: usize = RATE;
/// A phrase that runs this long without a pause is ended anyway, so little is left after stopping.
const LONGEST: usize = 15 * RATE;
/// Under a second there is too little to tell speech from the room.
const JUDGED: usize = 50;

#[derive(Default)]
pub struct Segmenter {
    levels: Vec<f64>,
    range: Option<(f64, f64)>,
    /// Windows measured when the range was last worked out.
    ranged: usize,
    /// Where the phrase being spoken began, in samples.
    from: usize,
    /// Quiet windows in a row, and whether anything was said since `from`.
    run: usize,
    spoken: bool,
}

impl Segmenter {
    /// Measures the audio heard since the last call. True when a phrase ended in it.
    pub fn step(&mut self, pcm: &[f32]) -> bool {
        let mut ended = false;
        while (self.levels.len() + 1) * WINDOW <= pcm.len() {
            let at = self.levels.len();
            let chunk = &pcm[at * WINDOW..(at + 1) * WINDOW];
            let level = (chunk.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / WINDOW as f64).sqrt();
            self.levels.push(level);
            if self.levels.len() >= JUDGED && (self.range.is_none() || self.levels.len() >= self.ranged + JUDGED) {
                let mut sorted = self.levels.clone();
                sorted.sort_by(f64::total_cmp);
                self.range = Some((sorted[sorted.len() / 10], sorted[sorted.len() * 9 / 10]));
                self.ranged = self.levels.len();
            }
            let Some((room, speech)) = self.range else { continue };
            // Quiet, for a pause, is under a fifth of the way from the room to speech.
            if level >= room + (speech - room) * 0.2 {
                (self.run, self.spoken) = (0, true);
            } else {
                self.run += 1;
            }
            let end = (at + 1) * WINDOW;
            let paused = self.run >= PAUSE && self.spoken && end - PAUSE / 2 * WINDOW >= self.from + PHRASE;
            if paused || end >= self.from + LONGEST {
                (self.from, self.run, self.spoken) = (end, 0, false);
                ended = true;
            }
        }
        ended
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Speech-like sound: loud and varying, unlike the room around it.
    fn said(seconds: f32) -> Vec<f32> {
        (0..(seconds * RATE as f32) as usize).map(|i| 0.3 * (i as f32 * 0.07).sin() * (0.7 + 0.3 * (i as f32 * 0.0008).sin())).collect()
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
    /// Feeds the audio 20 ms at a time, as the recording does, and counts the phrases ended.
    fn phrases(pcm: &[f32]) -> usize {
        let mut segmenter = Segmenter::default();
        (1..=pcm.len() / WINDOW).filter(|n| segmenter.step(&pcm[..n * WINDOW])).count()
    }

    #[test]
    fn a_phrase_ends_at_the_pause_after_it() {
        assert_eq!(phrases(&[room(0.3), said(2.0), room(0.7), said(1.0)].concat()), 1);
        assert_eq!(phrases(&[room(0.3), said(2.0), room(0.7), said(1.5), room(0.8)].concat()), 2);
    }

    #[test]
    fn short_breaths_inside_a_phrase_do_not_split_it() {
        assert_eq!(phrases(&[said(1.5), room(0.3), said(1.5), room(0.2), said(1.0)].concat()), 0);
    }

    #[test]
    fn a_word_too_short_to_stand_alone_waits_for_the_next() {
        assert_eq!(phrases(&[said(0.4), room(0.8), said(1.5)].concat()), 0);
    }

    #[test]
    fn a_long_run_without_a_pause_is_ended_anyway() {
        assert_eq!(phrases(&said(16.0)), 1);
    }

    #[test]
    fn silence_alone_is_no_phrase() {
        assert_eq!(phrases(&room(5.0)), 0);
    }
}

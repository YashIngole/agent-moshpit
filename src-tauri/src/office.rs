//! The state of every desk, as a function of what was seen at it.
//!
//! This module is pure. It does no I/O and never looks at a clock; the caller
//! passes in what a look at a terminal found, and the time. That is what makes
//! the rules below testable.

use crate::harness::{self, Harness};
use crate::model::{AgentView, Millis, Phase};
use crate::status::Signal;
use serde::{Deserialize, Serialize};

/// What is kept on disk so desks survive a restart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedDesk {
    pub id: String,
    /// The id of its program in the table.
    pub harness: String,
    pub title: String,
    /// True when the user named it; false when the name was made from the task.
    #[serde(default)]
    pub title_locked: bool,
    /// The name made for it when it was seated, which an emptied rename goes back to.
    #[serde(default)]
    pub auto_title: String,
    pub cwd: String,
    /// What the program calls the conversation, once known. Used to carry on.
    #[serde(default)]
    pub session: Option<String>,
    /// A session observed directly from its process, rather than an old folder/time guess.
    #[serde(default)]
    pub session_verified: bool,
    #[serde(default)]
    pub created_ms: Millis,
    #[serde(default)]
    pub launch: crate::launch::Options,
}

impl SavedDesk {
    pub fn resume_session(&self, kind: &Harness) -> Option<&str> {
        if kind.session == harness::SessionFrom::CodexRollouts && !self.session_verified {
            None
        } else {
            self.session.as_deref()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Desk {
    pub saved: SavedDesk,
    pub project: String,
    pub repo: String,
    pub branch: String,
    pub phase: Phase,
    pub since_ms: Millis,
    pub activity: String,
    /// Agent descriptions supplement observed status; never hide an approval or failure.
    reported_activity: String,
    agent_named: bool,
    pub running: bool,
    /// When its program was last started.
    pub started_ms: Millis,
    /// Has worked since anyone last looked; explicit completion can raise a flag.
    worked: bool,
    /// Has drawn its first screen and gone quiet once. Until then, printing is not work.
    settled: bool,
    /// Was handed something to do when its program started.
    tasked: bool,
    /// The user has typed into it since its program started.
    typed: bool,
    /// Went quiet before it began its task, waiting on a question of its own.
    asked_at_start: bool,
    /// A trust question has actually been observed on the startup screen.
    asks_trust: bool,
    /// Has gone quiet once since its program started.
    rested: bool,
    /// The user asked for its program to be ended, so its end is not a failure.
    leaving: bool,
    /// When its terminal was last in front of the user.
    seen_ms: Millis,
    /// It printed something since then: shown as a dot on its desk.
    unread: bool,
    pub look: u32,
}

/// A change of phase, reported so the caller can tell the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    pub id: String,
    pub title: String,
    pub from: Phase,
    pub to: Phase,
    pub note: String,
}

/// A program that ends with an error this soon after starting never got going.
const FALSE_START_MS: Millis = 10_000;
/// A program handed a task that goes quiet this soon after starting, before anyone
/// typed, has stopped to ask something first: trust this folder, sign in, update.
const OPENING_MS: Millis = 30_000;
/// ...after printing for no longer than this (the quiet after printing included).
/// Real work, even a quick answer, keeps a spinner going for longer.
const BRIEF_MS: Millis = 4_000;

#[derive(Debug, Default)]
pub struct Office {
    desks: Vec<Desk>,
}

impl Office {
    /// Everyone from the last run, asleep: nothing is started until its desk is opened.
    pub fn restore(saved: Vec<SavedDesk>, now: Millis) -> Self {
        let desks = saved.into_iter().filter(|s| !s.id.is_empty()).map(|saved| Desk::new(saved, Phase::Asleep, now)).collect();
        Office { desks }
    }

    pub fn saved(&self) -> Vec<SavedDesk> {
        self.desks.iter().map(|d| d.saved.clone()).collect()
    }

    pub fn desks(&self) -> &[Desk] {
        &self.desks
    }

    pub fn get(&self, id: &str) -> Option<&Desk> {
        self.desks.iter().find(|d| d.saved.id == id)
    }

    fn get_mut(&mut self, id: &str) -> Option<&mut Desk> {
        self.desks.iter_mut().find(|d| d.saved.id == id)
    }

    /// Whoever is busy enough that quitting should be asked about.
    pub fn busy(&self) -> usize {
        self.desks.iter().filter(|d| d.running && matches!(d.phase, Phase::Working | Phase::Quiet | Phase::NeedsYou | Phase::Starting)).count()
    }

    pub fn add(&mut self, saved: SavedDesk, now: Millis) {
        self.desks.push(Desk::new(saved, Phase::Starting, now));
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.desks.len();
        self.desks.retain(|d| d.saved.id != id);
        self.desks.len() != before
    }

    /// Its program has just been started. `tasked` says it was handed something to
    /// do, so what it prints from the first moment is work and not its opening screen.
    /// `asks_trust` says it will first ask whether to trust its folder.
    pub fn started(&mut self, id: &str, tasked: bool, asks_trust: bool, now: Millis) {
        if let Some(desk) = self.get_mut(id) {
            desk.running = true;
            desk.settled = tasked;
            desk.tasked = tasked;
            desk.typed = false;
            desk.asked_at_start = false;
            desk.asks_trust = asks_trust;
            desk.rested = false;
            desk.leaving = false;
            desk.started_ms = now;
            desk.seen_ms = now;
            desk.unread = false;
            desk.worked = false;
            desk.activity.clear();
            desk.reported_activity.clear();
            desk.turn(Phase::Starting, now);
        }
    }

    /// The user typed into its terminal.
    pub fn typed(&mut self, id: &str, now: Millis) {
        if let Some(desk) = self.get_mut(id) {
            desk.typed = true;
            desk.asked_at_start = false;
            if desk.asks_trust {
                // The folder question is answered: the hand comes down, and what it draws
                // next is its opening screen, or, given a task, the start of the work.
                desk.asks_trust = false;
                desk.settled = desk.tasked;
                desk.activity.clear();
                desk.turn(Phase::Starting, now);
            }
        }
    }

    /// A title no other desk has: "Claude in web", then "Claude in web 2".
    pub fn unique_title(&self, title: &str) -> String {
        let taken = |t: &str| self.desks.iter().any(|d| d.saved.title == t);
        if !taken(title) {
            return title.to_string();
        }
        (2..).map(|n| format!("{title} {n}")).find(|t| !taken(t)).unwrap_or_else(|| title.to_string())
    }

    /// The user asked for its program to be ended. Its end, when it comes, is expected.
    pub fn will_stop(&mut self, id: &str) {
        if let Some(desk) = self.get_mut(id) {
            desk.leaving = true;
        }
    }

    /// Its program could not be started at all.
    pub fn failed_to_start(&mut self, id: &str, why: &str, now: Millis) -> Option<Transition> {
        let desk = self.get_mut(id)?;
        desk.running = false;
        desk.activity = why.to_string();
        desk.turn(Phase::Failed, now)
    }

    /// Its program ended, by itself or because it was stopped.
    pub fn exited(&mut self, id: &str, ok: bool, now: Millis) -> Option<Transition> {
        let desk = self.get_mut(id)?;
        if !desk.running {
            return None;
        }
        desk.running = false;
        desk.worked = false;
        if !ok && !desk.leaving {
            desk.activity = if now.saturating_sub(desk.started_ms) < FALSE_START_MS {
                "It stopped with an error as soon as it started. Open the desk to see what it said."
            } else {
                "Its program stopped with an error. Open the desk to see what it said."
            }
            .into();
            desk.turn(Phase::Failed, now)
        } else {
            desk.activity.clear();
            desk.turn(Phase::Asleep, now)
        }
    }

    /// Startup predictions are not evidence. Refresh this from the actual terminal.
    pub fn observed_trust(&mut self, id: &str, asked: bool) {
        if let Some(desk) = self.get_mut(id) {
            if !desk.typed {
                if desk.asks_trust && !asked {
                    desk.activity.clear();
                    desk.settled = desk.tasked;
                }
                desk.asks_trust = asked;
            }
        }
    }

    /// One look at a running program. `watched` is true when its terminal is on
    /// screen in front of the user, who then needs no flag to know it finished.
    pub fn observe(&mut self, id: &str, signal: &Signal, watched: bool, now: Millis) -> Option<Transition> {
        let desk = self.get_mut(id)?;
        if !desk.running {
            return None;
        }
        // Asking whether to trust its folder: once it has drawn the question, it is
        // waiting for you, however it redraws, until the first key is pressed.
        if desk.asks_trust && !desk.typed {
            if *signal == Signal::Silent {
                return None;
            }
            desk.settled = true;
            desk.activity = "Asks whether to trust this folder".into();
            return desk.turn(Phase::NeedsYou, now);
        }
        if !desk.settled {
            match signal {
                // Drawing its opening screen is not work.
                Signal::Silent | Signal::Working => return None,
                _ => desk.settled = true,
            }
        }
        match signal {
            Signal::Silent => None,
            // Its opening screen, drawn in fits and starts (Codex takes eight seconds over
            // its own, with pauses): still not work, and its end is nothing to flag.
            Signal::Working if desk.drawing_its_opening(now) => None,
            Signal::Working => {
                desk.worked = true;
                desk.activity.clear();
                desk.turn(Phase::Working, now)
            }
            Signal::Asked(what) => {
                desk.worked = true;
                desk.activity = what.clone();
                desk.turn(Phase::NeedsYou, now)
            }
            Signal::Quiet | Signal::Finished => {
                // Handed a task, it went quiet at once and nobody has typed: it is asking
                // something before it starts, and stays that way until someone answers.
                if matches!(signal, Signal::Quiet) && desk.tasked && !desk.typed && (desk.asked_at_start || (!desk.rested && desk.paused_at_start(now))) {
                    desk.asked_at_start = true;
                    desk.activity = "Asked something before starting".into();
                    return desk.turn(Phase::NeedsYou, now);
                }
                desk.rested = true;
                if matches!(signal, Signal::Quiet) && desk.worked && desk.phase != Phase::Done {
                    desk.activity = "No recent output. It may still be working.".into();
                    return desk.turn(Phase::Quiet, now);
                }
                // Someone who has only just sat down, or was already resting, has finished nothing.
                let finished = desk.worked && matches!(desk.phase, Phase::Working | Phase::Quiet | Phase::NeedsYou);
                if desk.phase == Phase::Done && !watched {
                    return None;
                }
                desk.activity.clear();
                if finished && !watched {
                    desk.turn(Phase::Done, now)
                } else {
                    desk.worked = false;
                    desk.turn(Phase::Idle, now)
                }
            }
        }
    }

    /// What its program printed, as of now, and whether its terminal is in front of the
    /// user. Something printed since they last looked is news: a dot on its desk. Its
    /// opening screen, and a question about trusting its folder, are not.
    pub fn heard(&mut self, id: &str, printed: Millis, watched: bool, now: Millis) {
        let Some(desk) = self.get_mut(id) else { return };
        if watched {
            desk.seen_ms = now;
            desk.unread = false;
        } else if !desk.settled || desk.asks_trust || desk.drawing_its_opening(now) {
            desk.seen_ms = desk.seen_ms.max(printed);
        } else if printed > desk.seen_ms {
            desk.unread = true;
        }
    }

    /// The user looked at this desk's terminal: a finished flag comes down, and it is read.
    pub fn mark_seen(&mut self, id: &str, now: Millis) -> bool {
        let Some(desk) = self.get_mut(id) else {
            return false;
        };
        let read = std::mem::take(&mut desk.unread);
        desk.seen_ms = now;
        if desk.phase != Phase::Done {
            return read;
        }
        desk.worked = false;
        desk.turn(Phase::Idle, now);
        true
    }

    pub fn set_place(&mut self, id: &str, project: String, repo: String, branch: String) -> bool {
        let Some(desk) = self.get_mut(id) else {
            return false;
        };
        let changed = desk.project != project || desk.repo != repo || desk.branch != branch;
        desk.project = project;
        desk.repo = repo;
        desk.branch = branch;
        changed
    }

    pub fn unverify_session(&mut self, id: &str) {
        if let Some(desk) = self.get_mut(id) {
            desk.saved.session_verified = false;
        }
    }

    /// A name the program gave the conversation. The user's own name wins.
    pub fn suggest_title(&mut self, id: &str, title: &str) -> bool {
        let Some(desk) = self.get_mut(id) else {
            return false;
        };
        let title = harness::shorten(title, 60);
        if desk.saved.title_locked || desk.agent_named || title.is_empty() || desk.saved.title == title {
            return false;
        }
        desk.saved.title = title;
        true
    }

    /// Name a desk. An empty name gives it back the name it was given when it was
    /// seated (`fallback` for a desk from before that was kept), and the program may
    /// name it again.
    pub fn rename(&mut self, id: &str, title: &str, fallback: &str) -> bool {
        let Some(desk) = self.get_mut(id) else {
            return false;
        };
        desk.agent_named = false;
        let title = harness::shorten(title, 60);
        if title.is_empty() {
            let auto = if desk.saved.auto_title.is_empty() { harness::shorten(fallback, 60) } else { desk.saved.auto_title.clone() };
            let changed = desk.saved.title_locked || (!auto.is_empty() && desk.saved.title != auto);
            if !auto.is_empty() {
                desk.saved.title = auto;
            }
            desk.saved.title_locked = false;
            return changed;
        }
        desk.saved.title = title;
        desk.saved.title_locked = true;
        true
    }

    pub fn set_session(&mut self, id: &str, session: &str) -> bool {
        let Some(desk) = self.get_mut(id) else {
            return false;
        };
        if session.is_empty() || (desk.saved.session.as_deref() == Some(session) && desk.saved.session_verified) {
            return false;
        }
        desk.saved.session = Some(session.to_string());
        desk.saved.session_verified = true;
        true
    }

    pub fn agent_title(&mut self, id: &str, title: &str) {
        if let Some(desk) = self.get_mut(id) {
            if !desk.saved.title_locked {
                desk.saved.title = harness::shorten(title, 60);
                desk.agent_named = true;
            }
        }
    }

    pub fn agent_activity(&mut self, id: &str, activity: &str) {
        if let Some(desk) = self.get_mut(id) { desk.reported_activity = harness::shorten(activity, 140); }
    }

    pub fn agent_directory(&mut self, id: &str, cwd: String) -> bool {
        let Some(desk) = self.get_mut(id) else { return false };
        if desk.saved.cwd == cwd { return false; }
        desk.saved.cwd = cwd;
        true
    }

    pub fn views(&self, table: &[Harness]) -> Vec<AgentView> {
        self.desks
            .iter()
            .map(|d| {
                let kind = table.iter().find(|h| h.id == d.saved.harness);
                AgentView {
                    id: d.saved.id.clone(),
                    title: d.saved.title.clone(),
                    phase: d.phase,
                    activity: if matches!(d.phase, Phase::Working | Phase::Starting | Phase::Idle | Phase::Done) && !d.reported_activity.is_empty() { d.reported_activity.clone() } else { d.activity.clone() },
                    harness: d.saved.harness.clone(),
                    harness_name: kind.map_or_else(|| d.saved.harness.clone(), |h| h.name.clone()),
                    harness_tag: kind.map_or_else(|| d.saved.harness.clone(), |h| h.tag.clone()),
                    repo: d.repo.clone(),
                    project: d.project.clone(),
                    branch: d.branch.clone(),
                    cwd: d.saved.cwd.clone(),
                    since_ms: d.since_ms,
                    look: d.look,
                    running: d.running,
                    resumable: kind.is_some_and(|h| harness::can_resume(h, d.saved.resume_session(h))),
                    resume_scope: kind.map_or(harness::ResumeScope::None, |h| harness::resume_scope(h, d.saved.resume_session(h))),
                    resume_note: if kind.is_some_and(|h| h.session == harness::SessionFrom::CodexRollouts) && d.saved.session.is_some() && !d.saved.session_verified {
                        "This desk's saved Codex conversation could not be verified. Start Codex, then use /resume to choose it; its saved conversations are unchanged.".into()
                    } else {
                        String::new()
                    },
                    unread: d.unread,
                    launch: d.saved.launch.clone(),
                }
            })
            .collect()
    }
}

impl Desk {
    fn new(saved: SavedDesk, phase: Phase, now: Millis) -> Desk {
        let look = fnv1a(saved.id.as_bytes());
        let repo = crate::status::base_name(&saved.cwd);
        Desk {
            project: saved.cwd.clone(),
            saved,
            repo,
            branch: String::new(),
            phase,
            since_ms: now,
            activity: String::new(),
            reported_activity: String::new(),
            agent_named: false,
            running: false,
            started_ms: 0,
            worked: false,
            settled: false,
            tasked: false,
            typed: false,
            asked_at_start: false,
            asks_trust: false,
            rested: false,
            leaving: false,
            seen_ms: now,
            unread: false,
            look,
        }
    }

    /// Given nothing to do, with nobody having typed to it yet, and only just started:
    /// whatever it prints is its opening screen, even after a pause in the middle of it.
    fn drawing_its_opening(&self, now: Millis) -> bool {
        !self.tasked && !self.typed && now.saturating_sub(self.started_ms) < OPENING_MS
    }

    /// Quiet in the first moments of its run, after no more than a short burst of printing.
    fn paused_at_start(&self, now: Millis) -> bool {
        let early = now.saturating_sub(self.started_ms) < OPENING_MS;
        let brief = self.phase != Phase::Working || now.saturating_sub(self.since_ms) < BRIEF_MS;
        early && brief
    }

    /// Move to a phase, and say so if that is a change.
    fn turn(&mut self, to: Phase, now: Millis) -> Option<Transition> {
        if self.phase == to {
            return None;
        }
        let from = self.phase;
        self.phase = to;
        self.since_ms = now;
        Some(Transition { id: self.saved.id.clone(), title: self.saved.title.clone(), from, to, note: self.activity.clone() })
    }
}

/// Words a title does not end on: "Fix the total when a" reads as cut off.
const JOINERS: &[&str] = &["a", "an", "the", "and", "or", "but", "so", "if", "when", "while", "to", "of", "in", "on", "at", "by", "for", "from", "with", "as", "that", "is", "it", "its"];

/// A title for a desk when the user did not give one: the start of the task.
pub fn title_from(task: &str) -> String {
    let first_line = task.lines().next().unwrap_or(task);
    let mut words: Vec<&str> = first_line.split_whitespace().take(6).collect();
    // Never shorter than two words for it.
    while words.len() > 2 && words.last().is_some_and(|w| JOINERS.contains(&w.to_lowercase().trim_end_matches(['.', ',', ':', ';']))) {
        words.pop();
    }
    let title = harness::shorten(&words.join(" "), 40);
    title.trim_end_matches(['.', ',', ':', ';']).to_string()
}

fn fnv1a(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for b in bytes {
        hash ^= u32::from(*b);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desk(id: &str) -> SavedDesk {
        SavedDesk { id: id.into(), harness: "claude".into(), title: "Fix the total".into(), title_locked: false, auto_title: "Fix the total".into(), cwd: "C:/work/shop".into(), session: None, session_verified: false, created_ms: 1, launch: Default::default() }
    }

    fn office() -> Office {
        let mut office = Office::default();
        office.add(desk("a"), 100);
        office.started("a", true, false, 100);
        office
    }

    fn phase(office: &Office) -> Phase {
        office.get("a").unwrap().phase
    }

    #[test]
    fn legacy_codex_ids_are_kept_but_never_resumed_without_verification() {
        let mut saved = desk("a");
        saved.harness = "codex".into();
        saved.session = Some("legacy-session".into());
        let table = harness::built_in();
        let kind = table.iter().find(|h| h.id == "codex").unwrap();
        assert_eq!(saved.resume_session(kind), None);
        let mut office = Office::default();
        office.add(saved, 100);
        assert!(!office.views(&table)[0].resumable);
        assert!(!office.views(&table)[0].resume_note.is_empty());
        office.set_session("a", "terminal-verified-session");
        assert!(office.views(&table)[0].resumable);
        assert_eq!(office.saved()[0].session.as_deref(), Some("terminal-verified-session"));
    }

    #[test]
    fn a_trust_dialog_that_disappears_does_not_leave_a_false_badge() {
        let mut office = Office::default();
        office.add(desk("a"), 100);
        office.started("a", false, false, 100);
        office.observed_trust("a", true);
        office.observe("a", &Signal::Quiet, false, 1000);
        assert_eq!(phase(&office), Phase::NeedsYou);
        office.observed_trust("a", false);
        office.observe("a", &Signal::Quiet, false, 2000);
        assert_eq!(phase(&office), Phase::Idle);
    }

    #[test]
    fn agent_activity_supplements_work_but_never_hides_an_observed_question() {
        let mut office = office();
        office.observe("a", &Signal::Working, false, 1000);
        office.agent_activity("a", "Reading checkout tests");
        assert_eq!(phase(&office), Phase::Working);
        assert_eq!(office.views(&[])[0].activity, "Reading checkout tests");
        office.observe("a", &Signal::Asked("May I change the API?".into()), false, 2000);
        let question = office.views(&[])[0].activity.clone();
        office.agent_activity("a", "Pretend everything is fine");
        assert_eq!(phase(&office), Phase::NeedsYou);
        assert_eq!(office.views(&[])[0].activity, question);
    }

    #[test]
    fn work_that_nobody_watched_ends_with_a_flag() {
        let mut office = office();
        assert_eq!(office.observe("a", &Signal::Silent, false, 200), None);
        assert_eq!(phase(&office), Phase::Starting);
        let began = office.observe("a", &Signal::Working, false, 300).unwrap();
        assert_eq!((began.from, began.to), (Phase::Starting, Phase::Working));
        // Still working: nothing new to say.
        assert_eq!(office.observe("a", &Signal::Working, false, 900), None);
        let ended = office.observe("a", &Signal::Finished, false, 5_000).unwrap();
        assert_eq!((ended.from, ended.to), (Phase::Working, Phase::Done));
        // The flag stays up until someone looks.
        assert_eq!(office.observe("a", &Signal::Quiet, false, 9_000), None);
        assert!(office.mark_seen("a", 10_000));
        assert_eq!(phase(&office), Phase::Idle);
        assert!(!office.mark_seen("a", 10_001));
    }

    #[test]
    fn work_that_was_watched_ends_quietly() {
        let mut office = office();
        office.observe("a", &Signal::Working, true, 300);
        let ended = office.observe("a", &Signal::Finished, true, 5_000).unwrap();
        assert_eq!(ended.to, Phase::Idle);
        // And a flag that is up comes down when its terminal is brought in front.
        office.observe("a", &Signal::Working, false, 6_000);
        office.observe("a", &Signal::Finished, false, 9_000);
        assert_eq!(phase(&office), Phase::Done);
        office.observe("a", &Signal::Quiet, true, 9_500);
        assert_eq!(phase(&office), Phase::Idle);
    }

    #[test]
    fn someone_who_only_sat_down_has_finished_nothing() {
        let mut office = Office::default();
        office.add(desk("a"), 100);
        office.started("a", false, false, 100);
        let settled = office.observe("a", &Signal::Quiet, false, 2_000).unwrap();
        assert_eq!((settled.from, settled.to), (Phase::Starting, Phase::Idle));
    }

    #[test]
    fn handed_a_task_and_quiet_at_once_is_a_question_before_starting() {
        let mut office = office();
        // It drew a question (trust this folder?) and went quiet, and nobody has typed.
        office.observe("a", &Signal::Working, false, 300);
        let asked = office.observe("a", &Signal::Quiet, false, 3_000).unwrap();
        assert_eq!((asked.from, asked.to, asked.note.as_str()), (Phase::Working, Phase::NeedsYou, "Asked something before starting"));
        // The hand stays up however long it waits, and a look does not take it down.
        assert_eq!(office.observe("a", &Signal::Quiet, true, 120_000), None);
        assert!(!office.mark_seen("a", 120_500));
        // Answered: it gets on with the task, and the end of that is worth a flag.
        office.typed("a", 120_800);
        office.observe("a", &Signal::Working, false, 121_000);
        assert_eq!(office.observe("a", &Signal::Quiet, false, 140_000).unwrap().to, Phase::Quiet);
        assert_eq!(office.busy(), 1);
        assert_eq!(office.observe("a", &Signal::Finished, false, 141_000).unwrap().to, Phase::Done);
    }

    #[test]
    fn a_folder_question_is_a_raised_hand_until_answered_task_or_no_task() {
        for tasked in [false, true] {
            let mut office = Office::default();
            office.add(desk("a"), 100);
            office.started("a", tasked, true, 100);
            // Nothing drawn yet: still starting.
            assert_eq!(office.observe("a", &Signal::Silent, false, 200), None);
            let asked = office.observe("a", &Signal::Working, false, 400).unwrap();
            assert_eq!((asked.to, asked.note.as_str()), (Phase::NeedsYou, "Asks whether to trust this folder"));
            // Redrawn after a resize, looked at, left for minutes: still waiting.
            assert_eq!(office.observe("a", &Signal::Working, true, 2_000), None);
            assert_eq!(office.observe("a", &Signal::Quiet, true, 300_000), None);
            assert!(!office.mark_seen("a", 300_500));
            // Answered: the hand comes down at once.
            office.typed("a", 301_000);
            assert_eq!(phase(&office), Phase::Starting);
            office.observe("a", &Signal::Working, false, 301_500);
            let settled = office.observe("a", if tasked { &Signal::Finished } else { &Signal::Quiet }, false, 305_000).unwrap();
            // Given a task, that was the work; without one, only its opening screen.
            assert_eq!(settled.to, if tasked { Phase::Done } else { Phase::Idle }, "tasked: {tasked}");
        }
    }

    #[test]
    fn something_printed_unwatched_is_news_until_looked_at() {
        let mut office = Office::default();
        office.add(desk("a"), 100);
        office.started("a", false, false, 100);
        let unread = |office: &Office| office.views(&[])[0].unread;
        // Its opening screen, drawn while nobody watched, is not news.
        office.heard("a", 300, false, 400);
        office.observe("a", &Signal::Working, false, 400);
        office.heard("a", 300, false, 3_000);
        office.observe("a", &Signal::Quiet, false, 3_000);
        assert!(!unread(&office));
        // Nor is more of it after a pause, while nobody has typed to it.
        office.heard("a", 4_000, false, 4_100);
        assert!(!unread(&office));
        // What it prints once someone has is.
        office.typed("a", 4_500);
        office.heard("a", 5_000, false, 5_100);
        assert!(unread(&office));
        // Looked at, it is read; printed while looked at, it never was news.
        assert!(office.mark_seen("a", 6_000));
        assert!(!unread(&office));
        office.heard("a", 7_000, true, 7_100);
        office.heard("a", 7_000, false, 8_000);
        assert!(!unread(&office));
    }

    #[test]
    fn a_pause_in_output_is_not_completion() {
        let mut office = office();
        office.observe("a", &Signal::Working, false, 300);
        assert_eq!(office.observe("a", &Signal::Quiet, false, 9_000).unwrap().to, Phase::Quiet);
        assert_eq!(office.busy(), 1);
        assert!(!office.mark_seen("a", 9_050));
        assert_eq!(office.observe("a", &Signal::Finished, false, 9_100).unwrap().to, Phase::Done);
        // Once it has rested, quick work later is still work.
        office.mark_seen("a", 9_500);
        office.observe("a", &Signal::Working, false, 10_000);
        assert_eq!(office.observe("a", &Signal::Quiet, true, 12_600).unwrap().to, Phase::Quiet);
        assert_eq!(office.busy(), 1);
        assert_eq!(office.observe("a", &Signal::Finished, true, 13_000).unwrap().to, Phase::Idle);
        assert_eq!(office.busy(), 0);
    }

    #[test]
    fn two_desks_never_share_a_name() {
        let mut office = office();
        office.add(SavedDesk { id: "b".into(), title: "Fix the total 2".into(), ..desk("b") }, 100);
        assert_eq!(office.unique_title("Fix the total"), "Fix the total 3");
        assert_eq!(office.unique_title("Something new"), "Something new");
    }

    #[test]
    fn an_opening_screen_is_not_work() {
        let mut office = Office::default();
        office.add(desk("a"), 100);
        // Started with nothing to do: it draws its welcome, then waits.
        office.started("a", false, false, 100);
        assert_eq!(office.observe("a", &Signal::Working, false, 300), None);
        assert_eq!(phase(&office), Phase::Starting);
        assert_eq!(office.observe("a", &Signal::Quiet, false, 4_000).unwrap().to, Phase::Idle);
        // More of its opening screen after a pause, as Codex draws its own: still not work, and no flag.
        assert_eq!(office.observe("a", &Signal::Working, false, 6_000), None);
        assert_eq!(office.observe("a", &Signal::Quiet, false, 9_000), None);
        assert_eq!(phase(&office), Phase::Idle);
        // Once it has had time to open, printing is work, and its end is worth a flag.
        office.observe("a", &Signal::Working, false, 100 + OPENING_MS);
        assert_eq!(office.observe("a", &Signal::Quiet, false, 60_000).unwrap().to, Phase::Quiet);
        assert_eq!(office.observe("a", &Signal::Finished, false, 60_100).unwrap().to, Phase::Done);

        // And as soon as someone has typed to it, however early.
        let mut typed = Office::default();
        typed.add(desk("a"), 100);
        typed.started("a", false, false, 100);
        typed.observe("a", &Signal::Quiet, false, 3_000);
        typed.typed("a", 4_000);
        assert_eq!(typed.observe("a", &Signal::Working, false, 5_000).unwrap().to, Phase::Working);
    }

    #[test]
    fn a_question_raises_a_hand_and_says_what_for() {
        let mut office = office();
        office.observe("a", &Signal::Working, false, 300);
        let asked = office.observe("a", &Signal::Asked("Asked you a question".into()), false, 800).unwrap();
        assert_eq!((asked.to, asked.note.as_str()), (Phase::NeedsYou, "Asked you a question"));
        assert_eq!(office.get("a").unwrap().activity, "Asked you a question");
        // Answered: back to work, and the question is no longer shown.
        office.observe("a", &Signal::Working, true, 1_200);
        assert_eq!((phase(&office), office.get("a").unwrap().activity.as_str()), (Phase::Working, ""));
    }

    #[test]
    fn unexpected_errors_are_failures_even_after_startup() {
        let mut office = office();
        office.observe("a", &Signal::Working, false, 300);
        let left = office.exited("a", true, 60_000).unwrap();
        assert_eq!(left.to, Phase::Asleep);
        assert!(!office.get("a").unwrap().running);
        // Nothing more is seen at an empty desk.
        assert_eq!(office.observe("a", &Signal::Working, false, 61_000), None);
        assert_eq!(office.exited("a", true, 62_000), None);

        office.started("a", true, false, 70_000);
        let fell = office.exited("a", false, 71_000).unwrap();
        assert_eq!(fell.to, Phase::Failed);
        // Stopped by the user straight away: that is not a failure either.
        office.started("a", true, false, 75_000);
        office.will_stop("a");
        assert_eq!(office.exited("a", false, 75_500).unwrap().to, Phase::Asleep);
        // A late crash still needs attention.
        office.started("a", true, false, 80_000);
        assert_eq!(office.exited("a", false, 80_000 + FALSE_START_MS).unwrap().to, Phase::Failed);
    }

    #[test]
    fn desks_come_back_asleep_and_keep_what_matters() {
        let mut office = office();
        assert!(office.set_session("a", "s-1"));
        assert!(!office.set_session("a", "s-1"));
        assert!(office.suggest_title("a", "Checkout total, fixed"));
        assert!(office.rename("a", "Mine", ""));
        // Once the user has named it, the program's idea of a name is ignored.
        assert!(!office.suggest_title("a", "Something else"));
        let back = Office::restore(office.saved(), 500);
        let desk = back.get("a").unwrap();
        assert_eq!((desk.phase, desk.running, desk.saved.title.as_str(), desk.saved.session.as_deref()), (Phase::Asleep, false, "Mine", Some("s-1")));
        assert_eq!(desk.repo, "shop");
        // The same desk always has the same face.
        assert_eq!(desk.look, office.get("a").unwrap().look);
    }

    #[test]
    fn an_emptied_name_goes_back_to_the_one_it_was_given() {
        let mut office = office();
        assert!(office.rename("a", "Mine", "Claude in shop"));
        assert!(office.rename("a", "  ", "Claude in shop"));
        assert_eq!((office.get("a").unwrap().saved.title.as_str(), office.get("a").unwrap().saved.title_locked), ("Fix the total", false));
        // The program may name it again.
        assert!(office.suggest_title("a", "Checkout total, fixed"));
        assert!(office.rename("a", "", "Claude in shop"));
        assert_eq!(office.get("a").unwrap().saved.title, "Fix the total");
        // Already its own name, and not locked: nothing changes.
        assert!(!office.rename("a", "", "Claude in shop"));
        // A desk kept from before names were kept goes back to its program and folder.
        office.add(SavedDesk { id: "old".into(), auto_title: String::new(), title: "Named long ago".into(), title_locked: true, ..desk("old") }, 100);
        assert!(office.rename("old", "", "Claude in shop"));
        assert_eq!(office.get("old").unwrap().saved.title, "Claude in shop");
    }

    #[test]
    fn the_window_is_told_which_program_and_whether_it_carries_on() {
        let mut office = office();
        let table = harness::built_in();
        let view = office.views(&table).remove(0);
        assert_eq!((view.harness_name.as_str(), view.harness_tag.as_str(), view.resumable, view.running), ("Claude Code", "Claude", false, true));
        office.set_session("a", "s-1");
        assert!(office.views(&table)[0].resumable);
        assert_eq!(office.busy(), 1);
    }

    #[test]
    fn titles_come_from_the_start_of_the_task() {
        // Not cut off on a word that leads somewhere.
        assert_eq!(title_from("Fix the checkout total when a coupon is applied twice."), "Fix the checkout total");
        assert_eq!(title_from("Make the checkout total right when a coupon is applied"), "Make the checkout total right");
        assert_eq!(title_from("Refactor auth.\nThen add tests."), "Refactor auth");
        assert_eq!(title_from("Fix the"), "Fix the");
    }
}

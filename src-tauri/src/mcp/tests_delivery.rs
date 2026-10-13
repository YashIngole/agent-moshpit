//! Messages, receipts, and the ways a session is told of something new: with a tool's
//! answer, by a hook, by a push down its watch, and on waking. Each test drives the hub
//! as a session's MCP subprocess or hook does, over the real socket where there is one.
use super::tests::Fixture;
use super::*;
use std::io::BufReader;

fn message(f: &Fixture, body: &str, key: &str) -> String {
    f.call(&f.a, "send_message", json!({"session_id":f.b,"body":body,"request_key":key})).unwrap()["message"]["id"].as_str().unwrap().to_string()
}

/// How far an item got on its way to `desk`, in the words a sender is shown.
fn state(f: &Fixture, desk: &str, kind: Kind, id: &str) -> (String, String) {
    let mut journal = f.hub.state.lock().unwrap().journal.clone();
    let (revision, delivery) = journal.delivery_mut(desk, kind, id).unwrap();
    (delivery.state(revision).to_string(), delivery.notified_via.clone())
}

/// This start of the desk's program was given pushes, as a launch with them turned on is.
fn with_pushes(f: &Fixture, desk: &str) -> String {
    let token = f.hub.issue(desk).unwrap();
    f.hub.state.lock().unwrap().links.entry(desk.to_string()).or_default().channel = true;
    token
}

/// A connection the desk's subprocess holds open, as `bridge::watch` opens it.
fn watch(f: &Fixture, token: &str, after: u64, push: &str) -> BufReader<TcpStream> {
    let stream = TcpStream::connect(f.hub.address).unwrap();
    stream.set_read_timeout(Some(Duration::from_millis(1500))).unwrap();
    let mut reader = BufReader::new(stream);
    protocol::write_frame(reader.get_mut(), &json!({"token":token,"watch":{"after":after,"push":push,"client":"test-client 1.0","protocol":"2025-11-25"}})).unwrap();
    assert_eq!(expected(&mut reader), json!({"watching":true}));
    reader
}

/// The next thing written down a held connection, or None when it stays quiet or has closed.
/// The word the office says to a quiet watch now and then is not a thing.
fn frame(reader: &mut BufReader<TcpStream>) -> Option<Value> {
    loop {
        let value: Value = protocol::read_frame(reader, MAX_RESPONSE).ok().flatten().and_then(|bytes| serde_json::from_slice(&bytes).ok())?;
        if value.get("ping").is_none() {
            return Some(value);
        }
    }
}

/// The next thing written down a held connection, when one is expected: waited for however
/// slow the machine. Each item is saved to disk before it is written, and a disk can take
/// longer over that than a quiet connection is listened to (`watch` gives up a read after
/// a second and a half, which is right for hearing that nothing comes).
fn expected(reader: &mut BufReader<TcpStream>) -> Value {
    for _ in 0..20 {
        if let Some(value) = frame(reader) {
            return value;
        }
    }
    panic!("nothing was written down the held connection in half a minute")
}

fn until(mut ready: impl FnMut() -> bool) -> bool {
    for _ in 0..100 {
        if ready() { return true; }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn a_message_is_not_a_task_and_each_step_of_its_way_is_its_own_fact() {
    let f = Fixture::new();
    // Four unfinished tasks is the limit for work handed out. Findings are not work handed out.
    for n in 0..4 {
        f.call(&f.a, "send_task", json!({"session_id":f.b,"prompt":format!("Task {n}"),"request_key":format!("task-{n}")})).unwrap();
    }
    let refused = f.call(&f.a, "send_task", json!({"session_id":f.b,"prompt":"One too many","request_key":"task-4"})).unwrap_err();
    assert!(refused.contains("four unfinished") && refused.contains("send_message"), "{refused}");
    let sent = f.call(&f.a, "send_message", json!({"session_id":f.b,"body":"Finding: the cancel path still launches.","request_key":"finding-1"})).unwrap();
    let id = sent["message"]["id"].as_str().unwrap().to_string();
    // Saved, and the sender is told how its recipient will hear of it: not that it was read.
    assert_eq!((sent["delivery"]["state"].as_str(), sent["delivery"]["how"].as_str(), sent["reused"].as_bool()), (Some("saved"), Some("poll"), Some(false)));
    assert!(sent["delivery"]["note"].as_str().unwrap().starts_with("Saved."));
    // The same request again is the same message; the same key for other words is refused.
    assert_eq!(message(&f, "Finding: the cancel path still launches.", "finding-1"), id);
    assert!(f.call(&f.a, "send_message", json!({"session_id":f.b,"body":"Something else","request_key":"finding-1"})).is_err());
    assert_eq!(f.hub.state.lock().unwrap().journal.messages.len(), 1);

    // Nobody but the two of them reads it, and only the one it is for can acknowledge it.
    assert_eq!(state(&f, &f.b, Kind::Message, &id).0, "saved");
    assert!(f.call(&f.a, "acknowledge", json!({"id":id})).is_err());
    // Not shown yet, it cannot be acknowledged: an acknowledgement is of something seen.
    assert!(f.call(&f.b, "acknowledge", json!({"id":id})).unwrap_err().contains("not been shown"));

    // Listed whole in the inbox, it has been fetched. That accepts nothing and acknowledges nothing.
    let inbox = f.call(&f.b, "check_inbox", json!({})).unwrap();
    assert_eq!(inbox["messages"][0]["id"], id);
    assert_eq!((inbox["messages"][0]["body"].as_str(), inbox["messages"][0]["truncated"].as_bool()), (Some("Finding: the cancel path still launches."), Some(false)));
    assert_eq!(state(&f, &f.b, Kind::Message, &id).0, "fetched");
    assert!(inbox["assigned_tasks"].as_array().unwrap().iter().all(|t| t["status"] == "queued"));
    assert_eq!(inbox["assigned_tasks"].as_array().unwrap().len(), 4);

    // The sender sees how far it got, without being told anything it was not.
    let seen = f.call(&f.a, "check_inbox", json!({})).unwrap();
    assert_eq!((seen["sent_messages"][0]["id"].as_str(), seen["sent_messages"][0]["delivery"]["state"].as_str()), (Some(id.as_str()), Some("fetched")));

    // Acknowledged by the one it was for, once however often it is said.
    let acknowledged = f.call(&f.b, "acknowledge", json!({"id":id})).unwrap();
    assert_eq!((acknowledged["kind"].as_str(), acknowledged["up_to_date"].as_bool()), (Some("message"), Some(true)));
    assert_eq!(f.call(&f.b, "acknowledge", json!({"id":id})).unwrap(), acknowledged);
    let read = f.call(&f.a, "get_message", json!({"message_id":id})).unwrap();
    assert_eq!(read["message"]["delivery"]["state"], "acknowledged");
    assert!(read["message"]["delivery"]["acknowledged_ms"].as_u64().unwrap() > 0 && read["message"].get("request").is_none());
    // Acknowledged, it is no longer waiting in the inbox; the tasks still are, untouched.
    let after = f.call(&f.b, "check_inbox", json!({})).unwrap();
    assert!(after["messages"].as_array().unwrap().is_empty());
    assert_eq!(after["assigned_tasks"].as_array().unwrap().len(), 4);
    // It is kept: the office started again knows the same.
    let restored = Hub::start(f.handle.clone(), f.dir.join("coordination.json")).unwrap();
    let kept = restored.message(&f.a, &id).unwrap();
    assert_eq!((kept.delivery.state(kept.revision), kept.body.as_str()), ("acknowledged", "Finding: the cancel path still launches."));
    restored.shutdown();
}

#[test]
fn a_long_message_is_pointed_at_by_a_listing_and_fetched_by_reading_it() {
    let f = Fixture::new();
    let id = message(&f, &"A long finding. ".repeat(60), "long");
    let inbox = f.call(&f.b, "check_inbox", json!({})).unwrap();
    assert_eq!(inbox["messages"][0]["truncated"], true);
    // Its start was shown: that is being told of it, not having it.
    assert_eq!(state(&f, &f.b, Kind::Message, &id), ("notified".to_string(), "inbox".to_string()));
    let whole = f.call(&f.b, "get_message", json!({"message_id":id})).unwrap();
    assert_eq!(whole["message"]["body"].as_str().unwrap().len(), "A long finding. ".repeat(60).trim().len());
    assert_eq!(state(&f, &f.b, Kind::Message, &id).0, "fetched");
    // The sender reading its own message is not the recipient reading it.
    let mine = message(&f, "Second", "second");
    f.call(&f.a, "get_message", json!({"message_id":mine})).unwrap();
    assert_eq!(state(&f, &f.b, Kind::Message, &mine).0, "saved");
}

#[test]
fn something_new_is_said_once_with_the_answer_to_the_next_call() {
    let f = Fixture::new();
    let id = message(&f, "Please check the nested pane minimum.", "hint");
    // Whatever b asks next, it is told. Once.
    let answer = f.call(&f.b, "set_activity", json!({"activity":"Reading"})).unwrap();
    assert_eq!(answer["agent_moshpit_inbox"]["new"], 1);
    let note = answer["agent_moshpit_inbox"]["note"].as_str().unwrap();
    assert!(note.contains(&id) && note.contains("Please check the nested pane minimum.") && note.contains("not from the user"), "{note}");
    assert_eq!(state(&f, &f.b, Kind::Message, &id), ("notified".to_string(), "hint".to_string()));
    assert!(f.call(&f.b, "set_activity", json!({"activity":"Still reading"})).unwrap().get("agent_moshpit_inbox").is_none());
    // Read past, and unread a good while later: a program with no hooks has no other way to be reminded.
    {
        let mut state = f.hub.state.lock().unwrap();
        let mut journal = state.journal.clone();
        journal.messages[0].delivery.notified_ms -= mailbox::REMIND_MS + 1;
        f.hub.commit(&mut state, journal).unwrap();
    }
    let again = f.call(&f.b, "set_activity", json!({"activity":"Later"})).unwrap();
    assert!(again["agent_moshpit_inbox"]["note"].as_str().unwrap().contains(&id));
    assert!(f.call(&f.b, "set_activity", json!({"activity":"Later still"})).unwrap().get("agent_moshpit_inbox").is_none());
    // Told is not fetched: it is still in the inbox, and can be acknowledged from the telling.
    assert_eq!(f.call(&f.b, "acknowledge", json!({"id":id})).unwrap()["up_to_date"], true);
    // The sender's own calls say nothing of what it sent.
    message(&f, "Another", "hint-2");
    assert!(f.call(&f.a, "set_activity", json!({"activity":"Sending"})).unwrap().get("agent_moshpit_inbox").is_none());
}

#[test]
fn a_hook_is_told_what_is_new_once_and_asking_shows_that_hooks_run() {
    let f = Fixture::new();
    let token = f.hub.issue(&f.b).unwrap();
    let ask = |token: &str| {
        let stream = TcpStream::connect(f.hub.address).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut reader = BufReader::new(stream);
        protocol::write_frame(reader.get_mut(), &json!({"token":token,"hook":{"event":"PostToolBatch"}})).unwrap();
        expected(&mut reader)
    };
    // Nothing new: nothing to say. But it asked, and that is seen.
    assert_eq!(ask(&token), json!({"value":{"context":null}}));
    assert_eq!(f.hub.link_view(&f.b)["hooks"], "off");
    f.hub.state.lock().unwrap().links.entry(f.b.clone()).or_default().hooks = true;
    assert_eq!(f.hub.link_view(&f.b)["hooks"], "configured");
    let id = message(&f, "Stale activity on an unwatched desk.", "hook");
    // Its program has not connected yet: with no tools to read anything, it is told nothing, and nothing is claimed.
    assert_eq!(ask(&token), json!({"value":{"context":null}}));
    assert_eq!(state(&f, &f.b, Kind::Message, &id).0, "saved");
    let _connected = watch(&f, &token, 0, "none");
    assert!(until(|| f.hub.live(&f.b)));
    let told = ask(&token);
    let context = told["value"]["context"].as_str().unwrap();
    assert!(context.starts_with("Agent Moshpit: 1 new item") && context.contains(&id), "{context}");
    assert_eq!(state(&f, &f.b, Kind::Message, &id), ("notified".to_string(), "hook".to_string()));
    assert_eq!((f.hub.link_view(&f.b)["hooks"].as_str(), f.hub.link_view(&f.b)["hook_seen"].as_str()), (Some("seen"), Some("PostToolBatch")));
    // Said once at a step: the hook at the next step has nothing to add.
    assert_eq!(ask(&token), json!({"value":{"context":null}}));
    // A hook with somebody else's capability, or none, is told nothing.
    assert!(ask("not-a-token").get("error").is_some());

    // As the turn ends it is still unread: a note beside a tool's result is easily read past.
    // There it is said again, which keeps the turn going, and there it is said only once.
    let stop = |token: &str| {
        let stream = TcpStream::connect(f.hub.address).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut reader = BufReader::new(stream);
        protocol::write_frame(reader.get_mut(), &json!({"token":token,"hook":{"event":"Stop"}})).unwrap();
        expected(&mut reader)
    };
    let again = stop(&token);
    let context = again["value"]["context"].as_str().unwrap();
    assert!(context.starts_with("Agent Moshpit: 1 unread item") && context.contains("Before this turn ends") && context.contains(&id), "{context}");
    assert_eq!(state(&f, &f.b, Kind::Message, &id), ("notified".to_string(), "stop".to_string()));
    assert_eq!(stop(&token), json!({"value":{"context":null}}));
    assert_eq!(ask(&token), json!({"value":{"context":null}}));
    // Read at a step, there is nothing to say as the turn ends.
    let read = message(&f, "Read straight away", "hook-2");
    assert!(ask(&token)["value"]["context"].as_str().unwrap().contains(&read));
    f.call(&f.b, "get_message", json!({"message_id":read})).unwrap();
    assert_eq!(stop(&token), json!({"value":{"context":null}}));
}

#[test]
fn a_task_and_its_result_are_told_of_and_acknowledged_each_as_it_is_now() {
    let f = Fixture::new();
    let task = f.queued();
    // Assigned: news for the one it is assigned to.
    assert_eq!(f.call(&f.b, "set_activity", json!({"activity":"x"})).unwrap()["agent_moshpit_inbox"]["new"], 1);
    // Seen is not accepted. Acknowledging says it arrived; the task is still queued.
    f.call(&f.b, "acknowledge", json!({"id":task.id})).unwrap();
    assert_eq!(f.hub.task(&f.a, &task.id).unwrap().status, TaskStatus::Queued);
    // Accepting is the stronger thing, and counts as both.
    let other = f.call(&f.a, "send_task", json!({"session_id":f.b,"prompt":"A second review","request_key":"review-2"})).unwrap()["task"]["id"].as_str().unwrap().to_string();
    f.call(&f.b, "accept_task", json!({"task_id":other})).unwrap();
    assert_eq!(state(&f, &f.b, Kind::Task, &other).0, "acknowledged");

    // A result is news for whoever asked, and is not acknowledged before it is shown.
    f.call(&f.b, "report_result", json!({"task_id":task.id,"status":"blocked","summary":"Need the expected API shape"})).unwrap();
    assert!(f.call(&f.a, "acknowledge", json!({"id":task.id})).unwrap_err().contains("not been shown"));
    let hinted = f.call(&f.a, "set_activity", json!({"activity":"waiting"})).unwrap();
    assert!(hinted["agent_moshpit_inbox"]["note"].as_str().unwrap().contains("blocked: Need the expected API shape"));
    let first = f.call(&f.a, "acknowledge", json!({"id":task.id})).unwrap();
    assert_eq!((first["kind"].as_str(), first["up_to_date"].as_bool()), (Some("result"), Some(true)));
    let seen = first["acknowledged_revision"].as_u64().unwrap();

    // Reported on again, it is a new thing to be told of: what was acknowledged was the first.
    f.call(&f.b, "accept_task", json!({"task_id":task.id})).unwrap();
    f.call(&f.b, "report_result", json!({"task_id":task.id,"status":"completed","summary":"Reviewed the tests"})).unwrap();
    assert_eq!(state(&f, &f.a, Kind::Result, &task.id).0, "saved");
    // Saying again that the first was seen does not say the second was.
    let again = f.call(&f.a, "acknowledge", json!({"id":task.id,"revision":seen})).unwrap();
    assert_eq!((again["up_to_date"].as_bool(), again["acknowledged_revision"].as_u64()), (Some(false), Some(seen)));
    assert!(f.call(&f.a, "acknowledge", json!({"id":task.id,"revision":seen + 50})).is_err());
    // Read in full, it is fetched; then it can be acknowledged as it is.
    let read = f.call(&f.a, "get_task", json!({"task_id":task.id})).unwrap();
    assert_eq!((read["task"]["status"].as_str(), read["task"]["result_delivery"]["state"].as_str()), (Some("completed"), Some("fetched")));
    assert_eq!(f.call(&f.a, "acknowledge", json!({"id":task.id})).unwrap()["up_to_date"], true);
    // The one who did the work reading its own task acknowledges nothing for the one who asked.
    assert_eq!(f.call(&f.b, "get_task", json!({"task_id":other})).unwrap()["task"]["result_delivery"], Value::Null);
}

#[test]
fn a_session_started_with_pushes_is_written_to_as_things_come_and_again_after_a_new_start() {
    let f = Fixture::new();
    let token = with_pushes(&f, &f.b);
    let mut held = watch(&f, &token, 0, "channel");
    // Held open, the session's subprocess is known to be there, and what it said of itself is kept.
    assert!(until(|| f.hub.live(&f.b)));
    let link = f.hub.link_view(&f.b);
    assert_eq!((link["push"].as_str(), link["channel"].as_str(), link["client"].as_str()), (Some("channel"), Some("requested"), Some("test-client 1.0")));
    let sessions = f.call(&f.a, "list_sessions", json!({})).unwrap();
    let told = |id: &str| sessions["sessions"].as_array().unwrap().iter().find(|s| s["session"]["id"] == id).unwrap().clone();
    // Connected is the subprocess being there, not only the session having been started.
    assert_eq!((told(&f.b)["mcp_connected"].as_bool(), told(&f.a)["mcp_connected"].as_bool(), told(&f.a)["mcp"]["granted"].as_bool()), (Some(true), Some(false), Some(true)));

    let sent = f.call(&f.a, "send_message", json!({"session_id":f.b,"body":"Pushed finding","request_key":"push-1"})).unwrap();
    assert_eq!((sent["delivery"]["how"].as_str(), sent["delivery"]["recipient_live"].as_bool()), (Some("channel"), Some(true)));
    let id = sent["message"]["id"].as_str().unwrap().to_string();
    let event = expected(&mut held)["event"].clone();
    assert_eq!((event["kind"].as_str(), event["id"].as_str()), (Some("message"), Some(id.as_str())));
    assert!(event["channel"]["content"].as_str().unwrap().contains("Pushed finding"));
    assert_eq!(event["channel"]["meta"]["id"], id);
    // Written is not read. It is told of, by that way, and no more is claimed.
    assert_eq!(state(&f, &f.b, Kind::Message, &id), ("notified".to_string(), "channel".to_string()));
    assert_eq!(f.hub.link_view(&f.b)["channel"], "requested");
    // Just pushed, it is not said again with the next call's answer.
    assert!(f.call(&f.b, "set_activity", json!({"activity":"x"})).unwrap().get("agent_moshpit_inbox").is_none());
    // Nothing more is written for it while this connection lasts.
    let second = message(&f, "Second pushed finding", "push-2");
    assert_eq!(expected(&mut held)["event"]["id"], second);
    assert!(frame(&mut held).is_none());

    // The same subprocess connecting again, from where it had got to, is told nothing twice.
    let cursor = event["revision"].as_u64().unwrap() + 1;
    drop(held);
    let mut again = watch(&f, &token, cursor, "channel");
    assert!(frame(&mut again).is_none());
    drop(again);
    // A new subprocess (the program started again) knows of nothing: everything unread is said again.
    let mut fresh = watch(&f, &token, 0, "channel");
    let replayed: Vec<String> = (0..2).map(|_| expected(&mut fresh)["event"]["id"].as_str().unwrap().to_string()).collect();
    assert_eq!(replayed, [id.clone(), second.clone()]);
    // Read after a push: pushes do reach this session, and that is now known.
    f.call(&f.b, "get_message", json!({"message_id":id})).unwrap();
    assert_eq!(f.hub.link_view(&f.b)["channel"], "confirmed");
    f.call(&f.b, "acknowledge", json!({"id":second})).unwrap();
    drop(fresh);
    // Fetched or acknowledged, nothing is said again however new the subprocess.
    let mut quiet = watch(&f, &token, 0, "channel");
    assert!(frame(&mut quiet).is_none());

    // A result is pushed to whoever asked for the work.
    let asker = with_pushes(&f, &f.a);
    let mut asking = watch(&f, &asker, 0, "channel");
    let task = f.queued();
    f.call(&f.b, "report_result", json!({"task_id":task.id,"status":"completed","summary":"All reviewed"})).unwrap();
    let result = expected(&mut asking)["event"].clone();
    assert_eq!((result["kind"].as_str(), result["id"].as_str()), (Some("result"), Some(task.id.as_str())));
    assert!(result["channel"]["content"].as_str().unwrap().contains("completed: All reviewed"));
    // And the task itself was pushed to the one it was assigned to.
    assert_eq!(expected(&mut quiet)["event"]["kind"], "task");
}

#[test]
fn with_pushes_off_nothing_is_written_and_nothing_is_claimed_and_the_message_still_arrives() {
    let f = Fixture::new();
    // Started without pushes: the subprocess watches all the same, to be known alive.
    let token = f.hub.issue(&f.b).unwrap();
    let mut held = watch(&f, &token, 0, "none");
    assert!(until(|| f.hub.live(&f.b)));
    let sent = f.call(&f.a, "send_message", json!({"session_id":f.b,"body":"Not pushed","request_key":"off-1"})).unwrap();
    assert_eq!((sent["delivery"]["how"].as_str(), sent["delivery"]["state"].as_str()), (Some("poll"), Some("saved")));
    let id = sent["message"]["id"].as_str().unwrap().to_string();
    assert!(frame(&mut held).is_none());
    assert_eq!(state(&f, &f.b, Kind::Message, &id).0, "saved");
    // A subprocess that says it passes pushes on, in a session that was not started with them, is not believed.
    drop(held);
    let mut claims = watch(&f, &token, 0, "channel");
    assert!(frame(&mut claims).is_none());
    assert_eq!((state(&f, &f.b, Kind::Message, &id).0.as_str(), f.hub.link_view(&f.b)["channel"].as_str()), ("saved", Some("off")));
    // It is still there to be found, and found it is.
    assert_eq!(f.call(&f.b, "check_inbox", json!({})).unwrap()["messages"][0]["id"], id);
    assert_eq!(state(&f, &f.b, Kind::Message, &id).0, "fetched");
}

#[test]
fn a_push_that_goes_unread_is_said_again_by_a_way_that_is_known_to_arrive() {
    let f = Fixture::new();
    let token = with_pushes(&f, &f.b);
    let mut held = watch(&f, &token, 0, "channel");
    let id = message(&f, "Dropped on the floor", "grace");
    assert_eq!(expected(&mut held)["event"]["id"], id);
    // The program was not taking pushes after all. For a while that cannot be told from slowness.
    assert!(f.call(&f.b, "set_activity", json!({"activity":"x"})).unwrap().get("agent_moshpit_inbox").is_none());
    // Long enough unread, and the next call's answer says it, as it would have with no pushes at all.
    {
        let mut state = f.hub.state.lock().unwrap();
        let mut journal = state.journal.clone();
        journal.messages[0].delivery.notified_ms -= mailbox::PUSH_GRACE_MS + 1;
        f.hub.commit(&mut state, journal).unwrap();
    }
    let answer = f.call(&f.b, "set_activity", json!({"activity":"y"})).unwrap();
    assert!(answer["agent_moshpit_inbox"]["note"].as_str().unwrap().contains(&id));
    assert_eq!(state(&f, &f.b, Kind::Message, &id), ("notified".to_string(), "hint".to_string()));
    assert!(f.call(&f.b, "set_activity", json!({"activity":"z"})).unwrap().get("agent_moshpit_inbox").is_none());
    assert_eq!(f.hub.link_view(&f.b)["channel"], "requested");
}

#[test]
fn many_sessions_watching_do_not_keep_a_call_waiting_and_a_stopped_one_is_let_go() {
    let f = Fixture::new();
    let a = with_pushes(&f, &f.a);
    let b = with_pushes(&f, &f.b);
    // More connections held open than there are calls answered at once.
    let mut held: Vec<BufReader<TcpStream>> = (0..MAX_CALLS + 8).map(|n| watch(&f, if n % 2 == 0 { &a } else { &b }, 0, "channel")).collect();
    assert!(until(|| f.hub.held.load(Ordering::Relaxed) == MAX_CALLS + 8 && f.hub.connections.load(Ordering::Relaxed) == 0));
    let call = |token: &str| {
        let stream = TcpStream::connect(f.hub.address).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut reader = BufReader::new(stream);
        protocol::write_frame(reader.get_mut(), &json!({"token":token,"name":"check_inbox","arguments":{}})).unwrap();
        expected(&mut reader)
    };
    for _ in 0..MAX_CALLS + 4 {
        assert!(call(&a).get("value").is_some());
    }
    // What is new for a session is written once, down one of its own watches, and down nobody else's.
    let id = message(&f, "To b alone", "many");
    std::thread::sleep(Duration::from_millis(400));
    let mut heard = Vec::new();
    for (n, reader) in held.iter_mut().enumerate() {
        reader.get_ref().set_read_timeout(Some(Duration::from_millis(100))).unwrap();
        if let Some(event) = frame(reader) {
            assert_eq!(event["event"]["id"], id);
            heard.push(n);
        }
    }
    assert!(heard.len() == 1 && heard[0] % 2 == 1, "{heard:?}");
    // Stopped, a session's watches are closed: its capability is no longer its own.
    f.hub.revoke(&f.b);
    assert!(until(|| f.hub.held.load(Ordering::Relaxed) == (MAX_CALLS + 8) / 2));
    assert!(!f.hub.live(&f.b) && f.hub.live(&f.a));
    // And neither a watch nor a wait can be opened with it again.
    let refuse = |what: &str| {
        let stream = TcpStream::connect(f.hub.address).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut reader = BufReader::new(stream);
        protocol::write_frame(reader.get_mut(), &json!({"token":b,(what):{}})).unwrap();
        expected(&mut reader)
    };
    assert!(refuse("watch").get("error").is_some() && refuse("wait").get("error").is_some());
    assert!(call(&a).get("value").is_some());
    drop(held);
    assert!(until(|| f.hub.held.load(Ordering::Relaxed) == 0 && !f.hub.live(&f.a)));
}

#[test]
fn an_idle_session_that_waits_is_woken_once_and_only_its_newest_wait_is_kept() {
    let f = Fixture::new();
    let token = f.hub.issue(&f.b).unwrap();
    let wait = |token: &str| {
        let stream = TcpStream::connect(f.hub.address).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut reader = BufReader::new(stream);
        protocol::write_frame(reader.get_mut(), &json!({"token":token,"wait":{}})).unwrap();
        reader
    };
    // A session that was told of something as it started, before its program had connected, could only
    // say it has no tools. So one that waits is not woken until it has.
    let early = message(&f, "Sent before it connected", "early");
    let mut first = wait(&token);
    std::thread::sleep(Duration::from_millis(1500));
    assert_eq!(state(&f, &f.b, Kind::Message, &early).0, "saved");
    let _connected = watch(&f, &token, 0, "none");
    assert!(expected(&mut first)["value"]["context"].as_str().unwrap().contains(&early));
    f.call(&f.b, "acknowledge", json!({"id":early})).unwrap();
    // The turn before left one waiting; the turn after starts another. The older is let go, with nothing to say.
    let mut older = wait(&token);
    assert!(until(|| f.hub.held.load(Ordering::Relaxed) == 2));
    let mut newer = wait(&token);
    assert_eq!(expected(&mut older), json!({"value":{"context":null}}));
    // Nothing new: the newer goes on waiting, and is the only one that does.
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(f.hub.held.load(Ordering::Relaxed), 2);
    let id = message(&f, "Wake up: a finding", "wake");
    let woken = expected(&mut newer);
    assert!(woken["value"]["context"].as_str().unwrap().contains(&id), "{woken}");
    assert_eq!(state(&f, &f.b, Kind::Message, &id), ("notified".to_string(), "wake".to_string()));
    // Said: the hook at the next step has nothing to add, and the next wait goes on waiting.
    assert_eq!(f.hub.hook(&f.handle, &f.b, "PostToolBatch"), json!({"context":null}));
    let mut next = wait(&token);
    assert!(until(|| f.hub.held.load(Ordering::Relaxed) == 2));
    // A session that is stopped has its wait let go.
    f.hub.revoke(&f.b);
    assert_eq!(expected(&mut next), json!({"value":{"context":null}}));
}

#[test]
fn a_journal_from_before_messages_is_read_and_its_tasks_are_nobodys_news() {
    let f = Fixture::new();
    let path = f.dir.join("older.json");
    let old = json!({"tasks":[{"id":"t-old","request_key":"k","request":{},"from":f.a,"to":f.b,"prompt":"From 0.7","status":"queued","spawned":false,"summary":"","files":[],"validation":"","created_ms":1,"updated_ms":1}]});
    std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
    let hub = Hub::start(f.handle.clone(), path).unwrap();
    hub.issue(&f.a).unwrap();
    hub.issue(&f.b).unwrap();
    // It is still in the inbox, as it always was, and nothing is announced about it.
    let inbox = hub.call(&f.handle, &f.b, "check_inbox", &json!({})).unwrap();
    assert_eq!((inbox["assigned_tasks"][0]["id"].as_str(), inbox["assigned_tasks"][0]["delivery"]["state"].as_str()), (Some("t-old"), Some("saved")));
    assert!(hub.call(&f.handle, &f.b, "set_activity", &json!({"activity":"x"})).unwrap().get("agent_moshpit_inbox").is_none());
    assert!(hub.announce(&f.b, "hook").is_empty());
    // What is sent from now on has its place after it, and the file is written with both.
    hub.call(&f.handle, &f.a, "send_message", &json!({"session_id":f.b,"body":"New","request_key":"n"})).unwrap();
    let journal = hub.state.lock().unwrap().journal.clone();
    assert_eq!((journal.tasks.len(), journal.messages.len(), journal.revision), (1, 1, 1));
    hub.shutdown();
}

#[test]
fn one_session_cannot_wait_on_anothers_patience_or_fill_the_office_with_messages() {
    let f = Fixture::new();
    for n in 0..mailbox::MAX_WAITING {
        message(&f, &format!("Finding {n}"), &format!("many-{n}"));
    }
    let refused = f.call(&f.a, "send_message", json!({"session_id":f.b,"body":"One more","request_key":"too-many"})).unwrap_err();
    assert!(refused.contains("sixteen messages"), "{refused}");
    // Once some are acknowledged there is room again; the other direction was never limited by it.
    let first = f.hub.state.lock().unwrap().journal.messages[0].id.clone();
    f.call(&f.b, "check_inbox", json!({})).unwrap();
    f.call(&f.b, "acknowledge", json!({"id":first})).unwrap();
    f.call(&f.a, "send_message", json!({"session_id":f.b,"body":"One more","request_key":"too-many"})).unwrap();
    f.call(&f.b, "send_message", json!({"session_id":f.a,"body":"A reply","request_key":"reply","reply_to":first})).unwrap();
    // A message cannot be sent to oneself, to a desk in another project, or with a terminal's control codes in it.
    assert!(f.call(&f.a, "send_message", json!({"session_id":f.a,"body":"Me","request_key":"self"})).is_err());
    assert!(f.call(&f.a, "send_message", json!({"session_id":f.b,"body":"Bad\u{1b}[31m","request_key":"esc"})).is_err());
    let other = std::env::temp_dir().join(format!("moshpit-other-{}", random_id().unwrap()));
    std::fs::create_dir_all(&other).unwrap();
    let elsewhere = f.handle.new_agent(NewAgent { harness: "shell".into(), cwd: other.to_string_lossy().into_owned(), prompt: String::new(), title: String::new(), worktree: false, launch: Default::default() }, 80, 24).unwrap();
    f.hub.issue(&elsewhere).unwrap();
    assert!(f.call(&f.a, "send_message", json!({"session_id":elsewhere,"body":"Across","request_key":"across"})).unwrap_err().contains("different project"));
    // And a session in another project can neither read nor acknowledge what is not its own.
    assert!(f.call(&elsewhere, "get_message", json!({"message_id":first})).is_err());
    assert!(f.call(&elsewhere, "acknowledge", json!({"id":first})).is_err());
    f.handle.stop(&elsewhere);
    let _ = std::fs::remove_dir_all(other);
}

#[test]
fn the_window_is_shown_how_each_session_is_reached_and_how_far_each_item_got() {
    let f = Fixture::new();
    let id = message(&f, "For the window to show", "overview");
    f.queued();
    let view = f.hub.overview(&f.handle);
    assert_eq!(view["push"], json!({"hooks":true,"wake":true,"channels":false}));
    let desk = |id: &str| view["desks"].as_array().unwrap().iter().find(|d| d["id"] == id).unwrap().clone();
    assert_eq!((desk(&f.b)["waiting"].as_u64(), desk(&f.a)["waiting"].as_u64(), desk(&f.b)["link"]["push"].as_str()), (Some(2), Some(0), Some("poll")));
    let items = view["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    let shown = items.iter().find(|i| i["id"] == id).unwrap();
    assert_eq!((shown["kind"].as_str(), shown["delivery"]["state"].as_str(), shown["text"].as_str()), (Some("message"), Some("saved"), Some("For the window to show")));
    // Titles, for a person to read; never a capability.
    let text = view.to_string();
    assert!(shown["from_title"].is_string() && f.hub.state.lock().unwrap().grants.keys().all(|token| !text.contains(token.as_str())));
    // The user turns pushes on: it is what the next start of a session is given.
    f.hub.set_push(Push { hooks: true, wake: false, channels: true });
    assert_eq!(f.hub.overview(&f.handle)["push"], json!({"hooks":true,"wake":false,"channels":true}));
}

use super::*;

fn raw() -> RawEvent {
    RawEvent {
        gid: "040000008200E00074C5B7101A82E008".into(),
        entry_id: "00AB".into(),
        recurring: false,
        start: "2026-11-19T17:00:00.000Z".into(),
        end: "2026-11-19T17:30:00.000Z".into(),
        all_day: false,
        busy: 2,
        response: 3,
        meeting: 3,
        subject: " Weekly sync ".into(),
        location: "Microsoft Teams Meeting".into(),
        body: "Agenda: review the plan<https://example.com/plan>\r\n\r\n\r\nBring numbers.\r\n________________________________________________________________________________\r\nMicrosoft Teams Need help?<https://aka.ms/JoinTeamsMeeting>\r\nJoin the meeting now<https://teams.microsoft.com/l/meetup-join/19%3ameeting_abc%40thread.v2/0?context=x>\r\nMeeting ID: 123\r\n".into(),
    }
}

#[test]
fn reads_a_teams_meeting() {
    let e = parse_event(&raw()).unwrap();
    assert_eq!(e.id, "040000008200E00074C5B7101A82E008");
    assert_eq!(e.title, "Weekly sync");
    assert_eq!(e.starts_at, "2026-11-19T17:00:00.000Z");
    assert_eq!(e.duration_min, 30);
    assert!(!e.skip && !e.cancelled);
    assert_eq!(
        e.link_url.as_deref(),
        Some("https://teams.microsoft.com/l/meetup-join/19%3ameeting_abc%40thread.v2/0?context=x")
    );
    // The join block and inline link targets are dropped; the agenda stays.
    assert_eq!(
        e.agenda_md,
        "Location: Microsoft Teams Meeting\n\nAgenda: review the plan\n\nBring numbers."
    );
}

#[test]
fn occurrences_of_a_series_get_their_own_ids() {
    let mut r = raw();
    r.recurring = true;
    let a = parse_event(&r).unwrap();
    r.start = "2026-11-26T17:00:00.000Z".into();
    r.end = "2026-11-26T17:30:00.000Z".into();
    let b = parse_event(&r).unwrap();
    assert_ne!(a.id, b.id);
    assert!(a.id.starts_with("040000008200E00074C5B7101A82E008|"));
}

#[test]
fn skips_and_cancels_like_google() {
    for change in [
        |r: &mut RawEvent| r.all_day = true,
        |r: &mut RawEvent| r.response = 4,
        |r: &mut RawEvent| r.busy = 3,
        |r: &mut RawEvent| r.end = "2026-11-21T17:00:00.000Z".into(),
    ] {
        let mut r = raw();
        change(&mut r);
        assert!(parse_event(&r).unwrap().skip);
    }
    for status in [5, 7] {
        let mut r = raw();
        r.meeting = status;
        assert!(parse_event(&r).unwrap().cancelled);
    }
    let mut r = raw();
    r.start = "not a date".into();
    assert!(parse_event(&r).is_none());
}

#[test]
fn prefers_a_link_in_the_location() {
    assert_eq!(
        join_link("https://zoom.us/j/123 (Zoom)", "https://teams.microsoft.com/l/x").as_deref(),
        Some("https://zoom.us/j/123")
    );
    // Body links count only when they are a known meeting service.
    assert_eq!(join_link("Room 4", "See https://example.com/doc."), None);
    assert_eq!(
        join_link("", "Join: https://acme.zoom.us/j/99?pwd=x.").as_deref(),
        Some("https://acme.zoom.us/j/99?pwd=x")
    );
}

#[test]
fn reads_an_email_and_keeps_only_the_new_part() {
    let a = Account {
        id: "acct-1".into(),
        email: "mwilson@affirma.com".into(),
        store_id: "AB12".into(),
        project_id: None,
        mail_tasks: true,
        auto_add: false,
    };
    let v = json!({"id":"00FF","store_id":"AB12","from_name":"Dana Lee","from_address":"dana@acme.com","to":"Matthew Wilson",
        "subject":"","received":"2026-11-18T14:00:00.000Z","message_id":"<x@acme.com>",
        "body":"Can you send the deck by Friday?\r\n\r\nFrom: Matthew Wilson <mwilson@affirma.com>\r\nSent: Monday\r\nOld text"});
    let e = parse_mail(&v, &a).unwrap();
    assert!(e.outlook);
    assert_eq!(e.from, "Dana Lee <dana@acme.com>");
    assert_eq!(e.subject, "(no subject)");
    assert_eq!(e.mailbox, "mwilson@affirma.com");
    assert_eq!(e.body, "Can you send the deck by Friday?");
    assert_eq!(mail_ref(&e.account_id, &e.id), "acct-1:00FF");
    // Display names only when Outlook could not give addresses.
    assert_eq!(e.to, "Matthew Wilson");
    let mut v = v;
    v["to_addresses"] = json!("MWilson@affirma.com, dana@acme.com");
    assert_eq!(parse_mail(&v, &a).unwrap().to, "MWilson@affirma.com, dana@acme.com");
}

#[test]
fn checks_message_references() {
    assert_eq!(parse_ref("acct-1:00FF").unwrap(), ("acct-1", "00FF"));
    for bad in ["", "acct-1", "acct-1:", "acct 1:00FF", "acct-1:zz", "../x:00"] {
        assert!(parse_ref(bad).is_err(), "{bad}");
    }
}

#[test]
fn reads_the_bridge_answer() {
    assert_eq!(
        answer("noise\r\n{\"accounts\":[]}\r\n").unwrap()["accounts"],
        json!([])
    );
    let e = answer("{\"error\":\"Retrieving the COM class factory failed: 80040154 Class not registered\"}").unwrap_err();
    assert_eq!(e.message, "Classic Outlook is not installed on this computer.");
    assert!(answer("").is_err());
}

#[test]
fn the_script_fits_on_a_command_line() {
    // Windows limits a command line to 32,767 characters.
    assert!(encoded_script().len() < 30_000);
}

//! W-INT2 #44, W-LOOP: the venue's ALARM is the only clock its timed work has (DAG Phase 2), and
//! the work runs IN THE ALARM'S OWN TURN. A write that makes work due arms it; `alarm()` runs the
//! venue's jobs with the venue's own reads answered in process (no runner object, no request
//! back); while work is still due the alarm comes back one gap later, and once the work is done
//! it is cleared. The one cron the platform fires is the nightly the code knows
//! (`wrangler.toml` == `cloud::NIGHTLY_CRON`).

use crate::cron::timer::RUN_GAP_MS;
use crate::edge::mem::{answer_outbound, block_on, object_requests, sent};
use crate::edge::site::{post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::open_venue;
use crate::wire::Reply;
use serde_json::json;

fn token(site: &Site, t: &str) {
    let r = site.run(
        crate::services::venue::settings::set_setting,
        post(&format!("https://alpha.{PLATFORM_HOST}/api/owner/settings"), &json!({"key": "notify.telegram.token", "value": "123:abc"})).bearer(t).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
}

fn enqueue(site: &Site, id: &str, now: i64) {
    let place = crate::hubstore::Place::of_authorised(&site.ctx(&[]), "alpha").unwrap();
    let e = crate::outbox::Entry::new(id.into(), "telegram", "-4242".into(), "a line".into(), now);
    block_on(crate::outbox::enqueue(&place, &[e])).unwrap();
}

fn sends() -> usize {
    sent().iter().filter(|c| c.url().unwrap().path().ends_with("/sendMessage")).count()
}

#[test]
fn a_due_write_arms_the_alarm_and_the_alarm_drains_in_its_own_turn_until_the_work_is_done() {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    let host = site.world.host("alpha");
    assert!(host.alarm.get().is_none(), "nothing due, no alarm");

    let now = site.now_ms;
    enqueue(&site, "t-1:telegram", now);
    let at = host.alarm.get().expect("the write that made work due armed the alarm");
    assert!(at <= now + RUN_GAP_MS, "{at}");

    // THE ALARM FIRES while the rail is not configured (no bot token): the entry waits, and
    // the alarm is back one gap on. The run asked NO object for anything: its reads were calls.
    let before = object_requests();
    block_on(site.object("alpha").timer_alarm_in(&site.env(), at)).expect("the alarm ran");
    assert_eq!(object_requests() - before, 0, "the venue's own reads are answered in process");
    assert_eq!(host.alarm.get(), Some(at + RUN_GAP_MS), "still due: re-armed one gap later");
    assert_eq!(sends(), 0);

    // The owner sets the token; the next firing sends the line once, and the alarm CLEARS.
    token(&site, &t);
    answer_outbound(|_| Reply::from_json(&json!({"ok": true, "result": {"message_id": 1}})));
    block_on(site.object("alpha").timer_alarm_in(&site.env(), at + RUN_GAP_MS)).expect("the alarm ran");
    assert_eq!(sends(), 1);
    assert_eq!(host.alarm.get(), None, "idle: no alarm left set");
    // A late duplicate firing (at-least-once) sends nothing more.
    block_on(site.object("alpha").timer_alarm_in(&site.env(), at + 2 * RUN_GAP_MS)).expect("the alarm ran");
    assert_eq!(sends(), 1);
}

/// W-LOOP, THE COST OF ONE FIRING, counted: the same jobs run the old way (the runner object
/// `cron~alpha`, reading the venue back through its stub) and the new way (the venue's own
/// turn). The old firing was the alarm + the runner request + every read back; the new one is
/// the alarm alone. The live figure before was 11.9 (report §1.3).
#[test]
fn one_firing_costs_one_object_request_not_a_runner_and_its_reads_back() {
    let site = Site::new();
    let t = crate::cron::cadence_tests::link_without_hours(&site, "alpha");
    crate::cron::cadence_tests::idle_till();
    token(&site, &t);
    let at = site.now_ms + RUN_GAP_MS;
    enqueue(&site, "c-1:telegram", site.now_ms);
    let before = object_requests();
    block_on(crate::cron::run(&site.env(), "alpha", at));
    let runner = 2 + object_requests() - before; // + the alarm and the request to the runner
    enqueue(&site, "c-2:telegram", at);
    let before = object_requests();
    block_on(site.object("alpha").timer_alarm_in(&site.env(), at + RUN_GAP_MS)).expect("the alarm ran");
    let own = 1 + object_requests() - before; // + the alarm itself
    eprintln!("object requests per firing: runner path {runner}, own turn {own}");
    assert!(runner >= 6, "the old path's reads back were counted: {runner}");
    assert_eq!(own, 1, "one alarm, nothing read back");
}

#[test]
fn the_platform_fires_exactly_the_one_cron_the_code_runs() {
    let toml = include_str!("../../../wrangler.toml");
    let crons: Vec<&str> = toml
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("crons"))
        .flat_map(|l| l.split('"').skip(1).step_by(2))
        .collect();
    assert_eq!(crons, vec![crate::cloud::NIGHTLY_CRON], "wrangler.toml schedules a cron the code would only log as unknown");
}

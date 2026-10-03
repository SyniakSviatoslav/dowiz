//! W-INT2 #44: the venue's ALARM is the only clock its timed work has (DAG Phase 2). A write
//! that makes work due arms it; `alarm()` asks the venue's runner for one run, with the venue's
//! name and the alarm's instant; while work is still due the alarm comes back one gap later, and
//! once the work is done it is cleared. A runner that fails is an `Err` the platform retries,
//! and the alarm is left as it was. The one cron the platform fires is the nightly the code
//! knows (`wrangler.toml` == `cloud::NIGHTLY_CRON`).

use crate::cron::timer::RUN_GAP_MS;
use crate::edge::mem::{answer_outbound, block_on};
use crate::edge::site::{post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::open_venue;
use crate::wire::Reply;
use serde_json::json;

#[test]
fn a_due_write_arms_the_alarm_the_alarm_runs_the_runner_and_rearms_until_the_work_is_done() {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    let r = site.run(
        crate::services::venue::settings::set_setting,
        post(&format!("https://alpha.{PLATFORM_HOST}/api/owner/settings"), &json!({"key": "notify.telegram.token", "value": "123:abc"})).bearer(&t).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let host = site.world.host("alpha");
    assert!(host.alarm.get().is_none(), "nothing due, no alarm");

    let now = site.now_ms;
    let place = crate::hubstore::Place::of_authorised(&site.ctx(&[]), "alpha").unwrap();
    let e = crate::outbox::Entry::new("t-1:telegram".into(), "telegram", "-4242".into(), "a line".into(), now);
    block_on(crate::outbox::enqueue(&place, &[e])).unwrap();
    let at = host.alarm.get().expect("the write that made work due armed the alarm");
    assert!(at <= now + RUN_GAP_MS, "{at}");

    // THE ALARM FIRES while the work is still due (the runner answered but drained nothing):
    // the runner was asked for this venue at this instant, and the alarm is back one gap on.
    block_on(site.object("alpha").timer_alarm(at)).expect("the alarm ran");
    assert_eq!(host.runner_calls.borrow().as_slice(), &[("alpha".to_string(), at)]);
    assert_eq!(host.alarm.get(), Some(at + RUN_GAP_MS), "still due: re-armed one gap later");

    // A RUNNER THAT FAILS is an error the platform retries; the alarm is not touched.
    host.runner_status.set(500);
    assert!(block_on(site.object("alpha").timer_alarm(at + RUN_GAP_MS)).is_err());
    assert_eq!(host.alarm.get(), Some(at + RUN_GAP_MS));
    host.runner_status.set(200);

    // The runner's real work (`cron::run`, what `cron~alpha` does on that request) drains the
    // outbox; the next alarm finds nothing due and CLEARS itself.
    answer_outbound(|_| Reply::from_json(&json!({"ok": true, "result": {"message_id": 1}})));
    block_on(crate::cron::run(&site.env(), "alpha", at + RUN_GAP_MS));
    block_on(site.object("alpha").timer_alarm(at + 2 * RUN_GAP_MS)).expect("the alarm ran");
    assert_eq!(host.alarm.get(), None, "idle: no alarm left set");
    assert_eq!(host.runner_calls.borrow().len(), 3);
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

//! The staff doors' bodies (W-STRICT): sign-in, claim and password change are
//! read through `crate::body` and refuse a field they have no name for. Each
//! refusal beside the body the room app and the console actually send.

use super::{ClaimIn, LoginIn, PasswordIn};
use crate::body::from_text;

#[test]
fn the_console_login_body_passes_and_a_stray_field_does_not() {
    let ok: LoginIn = from_text(r#"{"email":"cook@x.al","password":"pw-pw-pw-1"}"#).unwrap();
    assert_eq!(ok.email, "cook@x.al");
    let e = crate::body::refusal::<LoginIn>(r#"{"email":"cook@x.al","password":"pw-pw-pw-1","remember":true}"#);
    assert!(e.contains("unknown field `remember`"), "{e}");
}

#[test]
fn the_claim_body_passes_and_a_stray_field_does_not() {
    let ok: ClaimIn = from_text(r#"{"email":"cook@x.al","code":"ABCD1234EFGH5678","password":"pw-pw-pw-1"}"#).unwrap();
    assert_eq!(ok.code, "ABCD1234EFGH5678");
    let e = crate::body::refusal::<ClaimIn>(r#"{"email":"cook@x.al","code":"A","password":"p","name":"Ana"}"#);
    assert!(e.contains("unknown field `name`"), "{e}");
}

#[test]
fn the_password_body_passes_and_a_misspelt_field_does_not() {
    let ok: PasswordIn = from_text(r#"{"email":"cook@x.al","old_password":"old-old-1","new_password":"new-new-1"}"#).unwrap();
    assert_eq!(ok.new_password, "new-new-1");
    // `newPassword` (camel) for `new_password`: live, the old parser dropped it
    // and answered "missing field" -- or worse, nothing, had it had a default.
    let e = crate::body::refusal::<PasswordIn>(r#"{"email":"cook@x.al","old_password":"old-old-1","newPassword":"new-new-1"}"#);
    assert!(e.contains("unknown field `newPassword`"), "{e}");
}

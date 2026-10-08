Hub images written on 2026-09-24 (commit cee20e61), frozen.

They were fixtures/decide/{amend.log,amend.stock,pay.log} until W-OCHAIN (b11f92dd,
2026-10-06) started writing the subject prefix into header cells: case.rs now builds
different bytes, so fixtures/decide/ was re-blessed (BEBOP_WASM_BLESS=1) and these
copies keep the OLD format under test -- bebop-store's
verify::tests::the_frozen_fixtures_pass_the_check loads them, because a check that
refused an image a venue already holds would be an outage on deploy.

Never re-bless or regenerate these. .img, not .log: *.log is in .gitignore.

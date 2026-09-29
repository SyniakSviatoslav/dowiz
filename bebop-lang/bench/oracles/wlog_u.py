# Oracle for gate `wlog_u`: u f 10000 8 5000 = events in the log after appending 5000 (n*k + 5000).
# See wlog_common.py (production dowiz-core decisions via bench/oracles/rust/src/bin/wlog.rs).
import wlog_common
wlog_common.value("u_events")

# Oracle for gate `wlog_i`: wlog.bp phase_ingest returns the injected illegal-step count.
# See wlog_common.py (production dowiz-core decisions via bench/oracles/rust/src/bin/wlog.rs).
import wlog_common
wlog_common.value("injected")

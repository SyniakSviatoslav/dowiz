-- Failures that must survive log sampling.
--
-- Workers Logs are sampled at one request in ten (`[observability]
-- head_sampling_rate`), which is right for the bill and wrong for a failure:
-- nine times out of ten the trace that carried the error is the one that was
-- dropped. And nothing on this box can read Workers Logs at all -- no token
-- here has the permission -- so an error that exists only there does not
-- exist. This table is the loud half: the venue's own console can read it, an
-- operator can query it, and it costs one D1 write on a path that is already
-- broken.
--
-- NOT A LOG. Only errors are written, never requests, and the nightly cron
-- deletes rows older than seven days.
CREATE TABLE IF NOT EXISTS worker_errors (
  id        INTEGER PRIMARY KEY AUTOINCREMENT,
  at_ms     INTEGER NOT NULL,  -- when the Worker saw it
  venue     TEXT,              -- the location id, when the failure had one; NULL for platform-wide
  place     TEXT NOT NULL,     -- WHERE in the code: "notify.telegram", "cloud.nightly", ...
  message   TEXT NOT NULL      -- the error as it was logged, truncated to 500 chars
);
CREATE INDEX IF NOT EXISTS worker_errors_recent ON worker_errors (at_ms DESC);
CREATE INDEX IF NOT EXISTS worker_errors_venue ON worker_errors (venue, at_ms DESC);

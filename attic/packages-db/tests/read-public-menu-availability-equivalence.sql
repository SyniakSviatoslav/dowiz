-- F4 equivalence proof: the set-based availability predicate (migration 064) must return
-- EXACTLY what the per-row plpgsql product_available_now() loop (migration 062) returns.
-- menu_schedule_matches is reused verbatim, so we only test the loop->EXISTS aggregation.
\set ON_ERROR_STOP on
DROP SCHEMA IF EXISTS t064 CASCADE;
CREATE SCHEMA t064;
SET search_path = t064;

-- verbatim from migration 062
CREATE FUNCTION menu_schedule_matches(
  p_mode text, p_start_minute int, p_end_minute int, p_days_of_week int[],
  p_starts_at timestamptz, p_ends_at timestamptz, p_local_now timestamp
) RETURNS boolean LANGUAGE sql IMMUTABLE AS $$
  SELECT CASE p_mode
    WHEN 'period' THEN
      (p_starts_at IS NULL OR p_local_now >= p_starts_at)
      AND (p_ends_at IS NULL OR p_local_now < p_ends_at)
    WHEN 'recurring' THEN
      (p_days_of_week IS NULL OR EXTRACT(DOW FROM p_local_now)::int = ANY(p_days_of_week))
      AND (p_start_minute IS NULL OR (EXTRACT(HOUR FROM p_local_now)::int * 60 + EXTRACT(MINUTE FROM p_local_now)::int) >= p_start_minute)
      AND (p_end_minute IS NULL OR (EXTRACT(HOUR FROM p_local_now)::int * 60 + EXTRACT(MINUTE FROM p_local_now)::int) < p_end_minute)
    ELSE
      (p_start_minute IS NULL OR (EXTRACT(HOUR FROM p_local_now)::int * 60 + EXTRACT(MINUTE FROM p_local_now)::int) >= p_start_minute)
      AND (p_end_minute IS NULL OR (EXTRACT(HOUR FROM p_local_now)::int * 60 + EXTRACT(MINUTE FROM p_local_now)::int) < p_end_minute)
  END;
$$;

CREATE TABLE menu_schedules (
  id serial PRIMARY KEY,
  location_id uuid,
  product_id uuid,
  category_id uuid,
  mode text NOT NULL DEFAULT 'daily',
  start_minute int, end_minute int, days_of_week int[],
  starts_at timestamptz, ends_at timestamptz,
  available boolean NOT NULL DEFAULT true
);

-- OLD logic (migration 062 loop), but v_local injected as a param so we can control "now".
CREATE FUNCTION pan_loop(p_product_id uuid, p_category_id uuid, v_local timestamp)
RETURNS boolean LANGUAGE plpgsql AS $$
DECLARE v_has_allow boolean := false; v_allow_hit boolean := false; r record;
BEGIN
  FOR r IN SELECT mode,start_minute,end_minute,days_of_week,starts_at,ends_at,available
           FROM menu_schedules WHERE product_id = p_product_id OR category_id = p_category_id LOOP
    IF menu_schedule_matches(r.mode,r.start_minute,r.end_minute,r.days_of_week,r.starts_at,r.ends_at,v_local) THEN
      IF r.available = false THEN RETURN false; END IF;
      v_allow_hit := true;
    END IF;
    IF r.available = true THEN v_has_allow := true; END IF;
  END LOOP;
  RETURN (NOT v_has_allow) OR v_allow_hit;
END; $$;

-- NEW logic (migration 064 set-based WHERE predicate), as a function for testing.
CREATE FUNCTION pan_set(p_product_id uuid, p_category_id uuid, v_local timestamp)
RETURNS boolean LANGUAGE sql AS $$
  SELECT
    NOT EXISTS (
      SELECT 1 FROM menu_schedules s
      WHERE (s.product_id = p_product_id OR s.category_id = p_category_id)
        AND s.available = false
        AND menu_schedule_matches(s.mode,s.start_minute,s.end_minute,s.days_of_week,s.starts_at,s.ends_at,v_local)
    )
    AND (
      NOT EXISTS (
        SELECT 1 FROM menu_schedules s2
        WHERE (s2.product_id = p_product_id OR s2.category_id = p_category_id) AND s2.available = true
      )
      OR EXISTS (
        SELECT 1 FROM menu_schedules s3
        WHERE (s3.product_id = p_product_id OR s3.category_id = p_category_id)
          AND s3.available = true
          AND menu_schedule_matches(s3.mode,s3.start_minute,s3.end_minute,s3.days_of_week,s3.starts_at,s3.ends_at,v_local)
      )
    );
$$;

-- Fixtures. v_local under test = 10:00 (600 min). active window=540..660, inactive=0..60.
-- products are addressed by deterministic uuids.
DO $$
DECLARE
  p1 uuid := '00000000-0000-0000-0000-000000000001'; -- no schedules
  p2 uuid := '00000000-0000-0000-0000-000000000002'; -- allow active
  p3 uuid := '00000000-0000-0000-0000-000000000003'; -- allow inactive
  p4 uuid := '00000000-0000-0000-0000-000000000004'; -- block active
  p5 uuid := '00000000-0000-0000-0000-000000000005'; -- block inactive
  p6 uuid := '00000000-0000-0000-0000-000000000006'; -- allow active + block active
  p7 uuid := '00000000-0000-0000-0000-000000000007'; -- allow active + block inactive
  p8 uuid := '00000000-0000-0000-0000-000000000008'; -- allow inactive + allow active
  c1 uuid := '00000000-0000-0000-0000-0000000000c1'; -- category-level allow active (p9 via category)
BEGIN
  INSERT INTO menu_schedules(product_id,available,start_minute,end_minute) VALUES
    (p2,true,540,660),(p3,true,0,60),
    (p6,true,540,660),(p7,true,540,660),
    (p8,true,0,60),(p8,true,540,660);
  INSERT INTO menu_schedules(product_id,available,start_minute,end_minute) VALUES
    (p4,false,540,660),(p5,false,0,60),
    (p6,false,540,660),(p7,false,0,60);
  INSERT INTO menu_schedules(category_id,available,start_minute,end_minute) VALUES
    (c1,true,540,660);
END $$;

-- Compare OLD vs NEW at two times (600=10:00 and 30=00:30) for every product + the category case.
WITH cases AS (
  SELECT * FROM (VALUES
    ('00000000-0000-0000-0000-000000000001'::uuid, NULL::uuid, 'p1 no-sched'),
    ('00000000-0000-0000-0000-000000000002'::uuid, NULL, 'p2 allow-active'),
    ('00000000-0000-0000-0000-000000000003'::uuid, NULL, 'p3 allow-inactive'),
    ('00000000-0000-0000-0000-000000000004'::uuid, NULL, 'p4 block-active'),
    ('00000000-0000-0000-0000-000000000005'::uuid, NULL, 'p5 block-inactive'),
    ('00000000-0000-0000-0000-000000000006'::uuid, NULL, 'p6 allow+block-active'),
    ('00000000-0000-0000-0000-000000000007'::uuid, NULL, 'p7 allow-active+block-inactive'),
    ('00000000-0000-0000-0000-000000000008'::uuid, NULL, 'p8 allow-inactive+allow-active'),
    ('00000000-0000-0000-0000-0000000000a9'::uuid, '00000000-0000-0000-0000-0000000000c1'::uuid, 'p9 category-allow-active')
  ) v(pid,cid,label)
),
times AS (SELECT unnest(ARRAY['2026-06-23 10:00:00','2026-06-23 00:30:00']::timestamp[]) AS v_local),
res AS (
  SELECT c.label, t.v_local,
         pan_loop(c.pid,c.cid,t.v_local) AS old_val,
         pan_set(c.pid,c.cid,t.v_local)  AS new_val
  FROM cases c CROSS JOIN times t
)
SELECT label, v_local::time AS at, old_val, new_val,
       CASE WHEN old_val IS NOT DISTINCT FROM new_val THEN 'OK' ELSE 'MISMATCH' END AS verdict
FROM res ORDER BY label, at;

-- Hard assertion: fail loudly if ANY case diverges.
DO $$
DECLARE n int;
BEGIN
  SELECT count(*) INTO n FROM (
    SELECT pan_loop(c.pid,c.cid,t.v_local) o, pan_set(c.pid,c.cid,t.v_local) s
    FROM (VALUES
      ('00000000-0000-0000-0000-000000000001'::uuid,NULL::uuid),
      ('00000000-0000-0000-0000-000000000002'::uuid,NULL),
      ('00000000-0000-0000-0000-000000000003'::uuid,NULL),
      ('00000000-0000-0000-0000-000000000004'::uuid,NULL),
      ('00000000-0000-0000-0000-000000000005'::uuid,NULL),
      ('00000000-0000-0000-0000-000000000006'::uuid,NULL),
      ('00000000-0000-0000-0000-000000000007'::uuid,NULL),
      ('00000000-0000-0000-0000-000000000008'::uuid,NULL),
      ('00000000-0000-0000-0000-0000000000a9'::uuid,'00000000-0000-0000-0000-0000000000c1'::uuid)
    ) c(pid,cid)
    CROSS JOIN (SELECT unnest(ARRAY['2026-06-23 10:00:00','2026-06-23 00:30:00']::timestamp[]) v_local) t
    WHERE pan_loop(c.pid,c.cid,t.v_local) IS DISTINCT FROM pan_set(c.pid,c.cid,t.v_local)
  ) x;
  IF n > 0 THEN RAISE EXCEPTION 'F4 EQUIVALENCE FAILED: % mismatching case(s)', n;
  ELSE RAISE NOTICE 'F4 EQUIVALENCE PROVEN: set-based predicate == product_available_now loop across all cases'; END IF;
END $$;

DROP SCHEMA t064 CASCADE;

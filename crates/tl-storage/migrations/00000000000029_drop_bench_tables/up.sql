-- Bench (raw-vs-guarded benchmark) feature removed; A/B comparison now lives in
-- the red-team report flow. Drop the parent-run + arm tables created in 28.
DROP TABLE IF EXISTS bench_run_arms;
DROP TABLE IF EXISTS bench_runs;

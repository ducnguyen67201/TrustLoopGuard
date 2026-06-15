DROP TABLE IF EXISTS bench_run_arms;
DROP TABLE IF EXISTS bench_runs;

ALTER TABLE redteam_job_results
    DROP COLUMN IF EXISTS trial_index,
    DROP COLUMN IF EXISTS kind,
    DROP COLUMN IF EXISTS track,
    DROP COLUMN IF EXISTS case_id;

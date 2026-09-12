-- Register the durable observer job type used for post-submission RPC checks.
INSERT INTO work.job_types (job_type, description) VALUES
    ('observe_trace', 'Observe eligible Solana traces through configured RPC routes')
ON CONFLICT (job_type) DO NOTHING;

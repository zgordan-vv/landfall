INSERT INTO work.job_types (job_type, description) VALUES
    ('observe_trace', 'Observe eligible trace status through configured RPC routes')
ON CONFLICT (job_type) DO NOTHING;

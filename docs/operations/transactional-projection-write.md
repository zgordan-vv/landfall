# Transactional projection write

`replace_trace_projection` upserts the reporting trace read model, removes its
previous typed submission-attempt children, inserts the new child set, and
commits once. Any failure rolls back both parent and children, preventing a
partially refreshed projection from being observed by API readers.

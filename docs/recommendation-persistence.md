# Recommendation persistence

`append_recommendations` stores a versioned recommendation and its diagnostic
and evidence edges in one transaction. Links use composite primary keys with
`ON CONFLICT DO NOTHING`, making replay idempotent while retaining prior rule
set versions for audit.

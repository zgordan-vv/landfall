# Trace detail API model

`TraceDetail` is the stable read-model envelope for one trace. It groups
attempts, independent observations, evidence, recommendations, and the
projection watermark used to explain read freshness. Generic section types let
the HTTP layer map domain projections without coupling the envelope to one
storage representation.

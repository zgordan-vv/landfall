# Request body limits

Ingestion applies two independent limits: compressed payloads are rejected
before decoding when `Content-Length` exceeds 256 KiB, and the extracted body
is capped at 2 MiB after decompression. The first limit bounds network and
decompression work; the second bounds allocator/JSON-parser exposure. Missing
or malformed `Content-Length` is not trusted as proof of size—the extractor
limit remains authoritative.

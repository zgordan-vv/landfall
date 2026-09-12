# Runtime container hardening

The final stage of `Dockerfile` already sets `USER 65532:65532`, so the server
does not run as root. For a read-only root filesystem, apply the restriction at
runtime because it is an orchestrator setting rather than an image build step:

```bash
docker run --rm \
  --read-only \
  --tmpfs /tmp:rw,noexec,nosuid,size=16m \
  --entrypoint /usr/bin/id \
  landfall-server:local
```

The command must report UID/GID `65532`. The temporary filesystem is intentionally
small and non-executable; it is available only for libraries that need a
scratch directory while the image itself remains immutable. Persistent state
belongs in PostgreSQL, not in the server container.

The application image contains no shell, package manager, source tree, or
compiler. Keep `--read-only` enabled in deployment manifests and add a narrowly
scoped writable mount only if a future feature demonstrates a concrete need.

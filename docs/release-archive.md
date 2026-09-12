# Release archive and provenance

Create a source archive, checksum, and small provenance record from a clean
commit:

```bash
LANDFALL_RELEASE_VERSION=v0.1.0 \
LANDFALL_RELEASE_DIR=dist/release \
./scripts/create-release-archive.sh
sha256sum -c dist/release/landfall-v0.1.0.tar.gz.sha256
```

The archive is produced by `git archive`, so ignored build outputs and local
secrets are excluded. The `.sha256` file detects transfer/corruption errors;
the provenance file records the commit, Dockerfile hash, and local image ID (or
`not-built`). Registry signatures and attestations should be added by the CI
release environment when an image is published.

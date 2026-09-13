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
secrets are excluded. The `.sha256` file detects transfer/corruption errors.
The provenance file records the commit, Dockerfile hash, requested image
reference, and local image ID when available. The Release workflow also records
the immutable published digest and attaches an SBOM/provenance attestation; see
the [release and deployment runbook](../operations/release-deployment.md).

#!/usr/bin/env bash

set -euo pipefail

image_reference="${1:-landfall-server:local}"

if ! command -v docker >/dev/null 2>&1; then
    printf 'Docker is required to inspect the server image.\n' >&2
    exit 127
fi

configured_user="$(docker image inspect --format '{{.Config.User}}' "$image_reference")"
entrypoint="$(docker image inspect --format '{{json .Config.Entrypoint}}' "$image_reference")"
license_label="$(docker image inspect --format '{{index .Config.Labels "org.opencontainers.image.licenses"}}' "$image_reference")"

if [[ "$configured_user" != "65532:65532" ]]; then
    printf 'Server image user mismatch: expected 65532:65532, found %s.\n' \
        "$configured_user" >&2
    exit 1
fi

if [[ "$entrypoint" != '["/usr/local/bin/landfall-server"]' ]]; then
    printf 'Server image entrypoint mismatch: %s.\n' "$entrypoint" >&2
    exit 1
fi

if [[ "$license_label" != "Apache-2.0" ]]; then
    printf 'Server image license label mismatch: %s.\n' "$license_label" >&2
    exit 1
fi

docker run --rm --entrypoint /usr/bin/test "$image_reference" -r /licenses/LICENSE

printf 'Server image uses the expected non-root user, entrypoint, and license metadata.\n'

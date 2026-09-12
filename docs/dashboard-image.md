# Dashboard assets in the server image

The Dockerfile has a dedicated `dashboard-builder` stage. It installs the
workspace with the locked pnpm dependency graph, builds the API client and then
builds `apps/dashboard` with Vite. The final server image copies only the
generated `apps/dashboard/dist` directory to `/opt/landfall/dashboard`; Node,
pnpm, TypeScript, and dashboard source files stay in the builder layer.

## Verification

```bash
docker build --tag landfall-server:local .
docker run --rm --entrypoint /bin/sh landfall-server:local \
  -c 'test -f /opt/landfall/dashboard/index.html'
```

The second command proves that the runtime image contains the static build. The
current server binary does not yet mount these files on an HTTP route; serving
or embedding them is a separate runtime integration task. Keeping the assets in
the image now makes the release artifact complete and deterministic.

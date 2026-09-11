# Observer HTTP transport

`ReqwestRouteClient` owns one pooled `reqwest::Client` per configured RPC
route. It uses Rustls-only TLS, a bounded request timeout, and reuses idle
connections. The transport verifies that the JSON-RPC endpoint passed by the
client matches the route configuration and reports only bounded status/error
text; response bodies from failed requests are not copied into diagnostics.

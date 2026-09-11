# Adaptive polling and route limits

`AdaptivePollPolicy` increases the delay between status checks exponentially,
clamps it to a configured maximum, and applies an additional multiplier after
rate limiting. `RouteRateLimiter` keeps an independent minimum request interval
for each RPC route, so a slow or throttled provider does not stall other routes.

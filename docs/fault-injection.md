# Fault-injection matrix

## Purpose

This matrix defines the expected safe response when one dependency fails. It is
an executable contract for the resilience suite, not proof that a live
dependency was stopped.

## Scenarios and expected evidence

| Injected failure | Expected response | Evidence to collect |
|---|---|---|
| Collector down | retry with backoff | no acknowledged event loss |
| Database down | halt writes, preserve error | readiness false; no false `202` |
| Projector crash | rehydrate durable jobs | job processed after restart |
| Observer rate limit | retry with adaptive delay | no tight retry loop |
| Corrupt RPC response | quarantine response | data-quality gap, no success conclusion |
| Disk pressure | halt writes | bounded error, process remains safe |
| Report worker crash | rehydrate report job | same job ID eventually completes |
| Retention failure | dry-run only | no partition dropped |

`fault_injection::expected_outcome` currently verifies the mapping above. A
separate integration harness must perform the real process/container failures
and capture the evidence listed in the final column.

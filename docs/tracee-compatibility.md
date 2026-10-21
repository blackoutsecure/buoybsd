# Tracee compatibility matrix

Reference reviewed at Tracee commit
[`2f9dc40c20b17c2ba27f6d92b25e62790bd48a62`](https://github.com/aquasecurity/tracee/tree/2f9dc40c20b17c2ba27f6d92b25e62790bd48a62).
Names and IDs below refer to its **protobuf EventId enum**, not raw
Linux syscall numbers. The BuoyBSD wire schema is independent.

| Tracee event / ID | BuoyBSD event / ID | Candidate FreeBSD source | Mapping / gaps | Privilege / tested FreeBSD | Evidence / status |
| --- | --- | --- | --- | --- | --- |
| `sched_process_exec` / 1015 | `process_exec` / 1000001 | DTrace `proc:::exec-success`; BSM exec records | Candidate timestamp, PID/PPID, credentials, path; argv completeness, stable process identity and success semantics unverified | Administrative native setup; no release tested | Synthetic replay tests only; **under-investigation** |
| `sched_process_exit` / 1016 | proposed `process_exit` / 1000002 | DTrace `proc:::exit`; BSM | SIGCHLD code is not automatically an exit status; identity/lifetime gaps | Unverified; no release tested | Not implemented; **under-investigation** |
| `open` / 3 | proposed `file_open` / 1000003 | BSM / DTrace | Linux syscall event not automatically equivalent to a FreeBSD open/access semantic event | Unverified; no release tested | Not implemented; **under-investigation** |
| `connect` / 43 | proposed `socket_connect` / 1000004 | BSM / socket probes | Attempt/completion, async sockets and address variants unverified | Unverified; no release tested | Not implemented; **under-investigation** |
| `bind` / 50 | proposed `socket_bind` / 1000005 | BSM / socket probes | Wildcards, families, selected ephemeral port and result unverified | Unverified; no release tested | Not implemented; **under-investigation** |
| `setuid` / 106 | proposed `credential_change` / 1000006 | BSM / syscall probes | Real/effective/saved IDs differ by API; no combined-equivalence claim | Unverified; no release tested | Not implemented; **under-investigation** |
| Linux namespace/cgroup/Kubernetes context | none | none | No fields invented from a jail or host process | Not applicable | **unsupported** |
| No one-to-one mapping | proposed jail/Capsicum/pf events; IDs unassigned | To be investigated | FreeBSD concepts deserve explicit fields, not Linux aliases | Unverified; no release tested | **under-investigation** |

Valid statuses: `equivalent`, `partially-equivalent`,
`FreeBSD-specific`, `unsupported`, `under-investigation`.
`FreeBSD-specific` is not a substitute for capture/test evidence.
No implemented native event currently earns that status.

## Implemented replay field contract

| BuoyBSD field | Input evidence | Tracee relationship |
| --- | --- | --- |
| Timestamp/name/ID | Explicit synthetic replay fields and local event catalogue | Similar concept, different event namespace |
| Process IDs and real/effective credentials | Optional observed-value fields, including present zero | Not a guarantee of Tracee process/thread identity semantics |
| Executable, command, argv | Optional fields and explicit completeness | Cached DTrace strings will not be split into a fabricated argv vector |
| Jail | Optional ID/name | Not a Tracee container or namespace |
| Raw bytes/source/sequence/warnings | Exact replay envelope and local ordering | BuoyBSD-specific provenance; not native telemetry |
| Policy/detection metadata | Tested local policy and Go detector | Conceptual alignment, not Tracee signature compatibility |

Replay-to-JSON integration is in
[`tests/integration/replay_test.go`](../tests/integration/replay_test.go).
It validates BuoyBSD's own contract, **not** native acquisition or
Tracee deserialization.

## Outputs and API

JSON, table, file, webhook, and one default broadcast stream are
conceptually aligned with Tracee's output separation. A webhook accepting
BuoyBSD JSON does not prove a Tracee consumer can decode it. Protobuf
package names, field numbers, event IDs, and JSON shapes differ.

Tracee's event uses policies, workload, event values, threat, and
detected-from messages. BuoyBSD deliberately keeps a smaller native
contract. Go templates, Forward and gRPC are not implemented.

Before marking any mapping equivalent, add a real capture, supported
release/privilege statement, exact field mapping, explicit missing and
non-equivalent fields, automated interoperability test against a pinned
Tracee schema/consumer, and documented semantic review.

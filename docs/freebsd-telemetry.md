# FreeBSD telemetry feasibility

This is a documentation-grounded assessment, **not** a record of a
successful native experiment. No FreeBSD host or captured telemetry
was available during bootstrap.

## Process-execution candidates

| Source | Documented observations | Gaps / enrichment | Privileges and loss |
| --- | --- | --- | --- |
| DTrace `proc:::exec`, `exec-success`, `exec-failure` | Attempt filename, success/failure probes; translated current-process PID, PPID, real/effective credentials, jail ID, cached argument string | Do not assume attempt and success paths are identical. `pr_psargs` is a cached string, not a complete argv vector; bounded by `kern.ps_arg_cache_limit`. Exact executable identity and start time need verification. | DTrace device/module access normally requires privileged setup. Buffers can drop; capture diagnostics and counters. |
| Audit trail / auditpipe BSM | Selected audit event, timestamp, subject credentials/identity, object/argument tokens, success/failure | Verify actual `execve`/`fexecve` records and token variants on the target release. Argument/environment policy is configurable, and missing tokens are not zero values. PPID, jail, path resolution, argv completeness need evidence. | Audit setup and device permissions are administrative. Trail storage and live pipe have different reliability; auditpipe can drop on queue overflow and undersized reads. |
| `kqueue` process notifications | Lifecycle notification for registered processes | Not a global execution feed; registration races and absent exec metadata preclude primary acquisition. | Limited to observable/registered processes; source-specific restrictions. |
| `sysctl` / process snapshots | Context about processes that still exist | Snapshots are enrichment, not a lossless event stream; short-lived processes and PID reuse race lookups. | Per-field visibility varies with credentials and jail/security settings. |

Prefer DTrace for the **smallest feasibility experiment** because the
documented `proc` provider exposes execution outcomes and process
context without first implementing a BSM parser. Auditpipe remains a
strong candidate for the eventual sensor; choose after actual captures
and loss measurements, not because a documentation table looks complete.

## Smallest manual, isolated experiment

Use a disposable VM running a release currently supported by the
[FreeBSD Security Team](https://www.freebsd.org/security/#sup).
Record the exact release, architecture, kernel configuration, Go
version, probe listing, and privilege requirements. No release is
runtime-certified by this repository yet.

1. As an ordinary user, record `freebsd-version -kru`, `uname -a`,
   `sysctl kern.ps_arg_cache_limit`, and whether DTrace/audit facilities
   are present. Do not copy host names or other sensitive identifiers
   into published fixtures.
2. With an administrator's explicit approval, inspect available probes:
   `dtrace -l -P proc`. If unavailable, stop and document the setup
   requirement; do not automatically load modules or change host policy.
3. In a dedicated terminal, run the documented `proc:::exec-success`
   probe printing only PID, PPID, real/effective UID/GID, jail ID, and
   `pr_psargs`. Use the installed `psinfo.d` translator, not guessed
   kernel structures or Solaris-specific argument assumptions.
4. As an ordinary user in a second terminal, execute `/usr/bin/true`
   once, then a harmless deliberately missing executable. Verify that
   success is not conflated with an attempt/failure. Interrupt only
   this probe with Ctrl+C; keep its stdout and stderr separately.
5. Repeat with a long harmless argument and a short-lived child.
   Establish what is truncated/unavailable. Record DTrace drop/error
   diagnostics, timestamps, ordering limitations, and whether output
   fields refer to the pre-exec or post-exec image.
6. Sanitize and commit the real capture **with a manifest**: exact
   command/script, release, privileges, raw encoding, redactions,
   timestamp provenance, truncation, diagnostics, and observed loss.
   Never retain process environments or unrelated host activity.
7. Write a source parser/normalizer test and review semantics before
   implementing a continuous collector or claiming compatibility.

This document intentionally does not ship an untested DTrace script
or a root-running integration test. The first experiment validates the
script and capture format before either becomes production code.

If DTrace is unavailable, assess audit in a separate disposable VM:
inspect existing configuration and local audit event definitions first.
Do not enable audit classes, log environments, alter device ownership,
or read system-wide records without explicit authorization. For a
future auditpipe reader, query maximum record size before allocation,
bound queue settings, and expose kernel pipe drop counters.

## Later event feasibility

| Event | Candidate | Required validation |
| --- | --- | --- |
| Process exit | DTrace `proc:::exit`, BSM | Probe argument is a SIGCHLD code, not necessarily a numeric exit status. |
| File open/access | Audit events, DTrace syscall probes | Success/failure, descriptor/path resolution, relative paths and jail roots. |
| Outbound connect | BSM / native socket probes | IPv4/IPv6, asynchronous completion, peer identity, success semantics. |
| Bind | BSM / native socket probes | Wildcards, ephemeral ports, return status, socket family. |
| Credential change | BSM / syscall probes | Real/effective/saved IDs and failed attempts. |
| Jail, Capsicum, pf, module activity | Source-specific FreeBSD interfaces | Availability, permissions, event loss and stable semantics before advertising support. |

devd is not a generic process-execution feed. pf logs do not stand in
for all socket syscalls. Capsicum is a security facility, not proof of
a ready-made violation event source.

## Primary references

- [FreeBSD `dtrace_proc(4)`](https://man.freebsd.org/cgi/man.cgi?query=dtrace_proc&sektion=4)
- [FreeBSD DTrace handbook](https://docs.freebsd.org/en/books/handbook/dtrace/)
- [FreeBSD audit handbook](https://docs.freebsd.org/en/books/handbook/audit/)
- [FreeBSD `auditpipe(4)`](https://man.freebsd.org/cgi/man.cgi?query=auditpipe&sektion=4)
- [FreeBSD `kqueue(2)`](https://man.freebsd.org/cgi/man.cgi?query=kqueue&sektion=2)
- [FreeBSD `sysctl(3)`](https://man.freebsd.org/cgi/man.cgi?query=sysctl&sektion=3)

Consult the target release's own manuals and headers. Online manuals
can describe a different release and do not replace runtime evidence.

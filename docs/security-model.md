# Security model

## Trust boundaries

Replay files and future native telemetry are untrusted. Configuration
is administrator-controlled but still strictly validated. Outputs
may carry sensitive host/process information; destinations are trusted
recipients selected by that administrator.

The prototype is not a security boundary or a tamper-resistant audit
service. Root on a monitored host can affect telemetry and the sensor.
There is no remote management listener.

## Input and resource safety

Bound record bytes, string/argument sizes, nesting, queue capacity,
configuration size, HTTP retries, and request duration. Reject invalid
UTF-8, duplicate keys, unknown fields, unsupported events, invalid
numbers, and false evidence claims. Do not invoke a shell with observed
values. Missing values remain absent.

Replay runs without root. A future native collector must list its
device, module, audit, jail, and credential permissions and separate
privileged acquisition from ordinary processing where practical.

## Output safety

- Diagnostic JSON logs use stderr and never include raw records,
  process environments, header values, or endpoint URLs.
- Event JSON intentionally includes raw bytes and configured/observed
  arguments. Base64 is not encryption. Use restrictive file ownership
  and retention controls.
- Table fields are ASCII-quoted to prevent ANSI/control injection.
- JSON files are opened append-only with mode 0600 for newly created
  files on POSIX. Existing files keep their existing permissions;
  Windows access is governed by ACLs. No automatic rotation is provided.
- Configuration/replay files cannot also be file destinations, including
  aliases resolved to the same existing file.
- Webhooks verify TLS using Go's system trust, do not follow redirects,
  reject URL userinfo/fragments, and never echo secret-bearing URLs or
  headers in errors. HTTP requires explicit opt-in and a loopback host.
- Retries are finite (0-5) and only for network failures, HTTP 429, or
  5xx responses. Redirects/other 4xx responses fail. Duplicate delivery
  is possible. Response bodies are discarded with a bounded read.

Do not use custom headers as a substitute for transport security.
Avoid secrets in URL paths/query parameters; even though diagnostics
omit URLs, a recipient/proxy can log them.

## Testing and deployment

Unit/replay tests need no privileges and use sanitized synthetic input.
Webhook tests use local ephemeral endpoints, not external services.
Native/privileged tests require explicit opt-in and isolated hosts;
never silently enable audit, load modules, change pf, or change
production telemetry configuration.

Protect configuration containing headers, and account for terminal,
CI-log, file, and receiver retention. No environment harvesting occurs.
Report security problems privately via [SECURITY.md](../SECURITY.md).

# Fixture provenance

`process_exec.synthetic.ndjson` is an independently authored synthetic
example of the **BuoyBSD replay envelope**, version `buoybsd.replay.v1`.
It is not output captured from FreeBSD, DTrace, auditpipe, or Tracee.
Its timestamps, identifiers, and fixture host name are test values.
It contains no production data, environment, customer data, or secrets.

The two records exercise present-zero credentials and a shell detector,
then a non-shell execution with missing optional identity fields.
They do not validate native capture, enrichment, privilege needs, event
ordering, or Tracee equivalence.

Native fixtures must be actual sanitized captures with a manifest
recording source command/script, OS/kernel release, architecture,
privileges, raw format/encoding, redactions, timestamp provenance,
truncation, diagnostics and drop counters. Keep native raw fixtures
separate from this replay format and never relabel synthetic input.

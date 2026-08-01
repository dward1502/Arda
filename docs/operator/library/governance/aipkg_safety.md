# AIPKG Safety and Governance Notes

This document captures the safety intention of AIPKG governance gates.

These gates do not replace human supervision for safety-critical decisions.
For a compliant AIPKG receipt chain, however, every declared governance gate
is mandatory and missing or failed evidence is rejected.

## Zero work preflight

Preflight must be allowed without executing the package. Zero work must remain
zero work. The default preflight receipt expires after 15 minutes; production
callers must supply its signature.

## Evidence integrity

The contract layer does not treat an enabled manifest flag as proof that a
gate ran. Executors must supply observed Triad, Bacon-lite, JouleWork, and Love
outcomes. Receipt-chain validation also requires matching package identity and
digest, a successful exit code, valid RFC3339 timestamps, and signatures when
the manifest requires them.

## Operator responsibility

Operators remain accountable for final approval, signing, and attestation.
Automation can reduce toil; it cannot assume liability.

## Health/biomagnetic context

This surface does not evaluate medical data. Any bio-related or health input is
treated as advisory context only. Separate clinical systems should be used for
measurement and diagnosis.

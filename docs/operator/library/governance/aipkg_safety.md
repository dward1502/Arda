# AIPKG Safety and Governance Notes

This document captures the safety intention of AIPKG governance gates.

These gates are reminders and optional programmatic checks. They do not
replace human supervision for safety-critical decisions.

## Zero work preflight

Preflight must be allowed without executing the package. Zero work must remain
zero work.

## Operator responsibility

Operators remain accountable for final approval, signing, and attestation.
Automation can reduce toil; it cannot assume liability.

## Health/biomagnetic context

This surface does not evaluate medical data. Any bio-related or health input is
treated as advisory context only. Separate clinical systems should be used for
measurement and diagnosis.

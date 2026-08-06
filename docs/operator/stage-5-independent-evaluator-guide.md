# Stage 5 Independent Evaluator Guide

Use this protocol to close `USABILITY-EVAL-001`. The evaluator must not be an
Arda implementer and must work from the native launcher and Workbench surfaces,
not source files, raw state, or an implementer explanation.

This packet prepares the evaluation; it is not evidence that the gate passed.
Only a completed timestamped record from a qualifying non-author evaluator can
close the gate.

## Safety and scope

- Use an isolated account or clean supported-profile VM and a disposable fixture.
- Use only the exact signed candidate assets named in the release manifest.
- Do not enter production credentials, private source, health information, or
  other sensitive data.
- Do not let the evaluator inspect the Arda repository, `core/state`, raw JSON
  receipts, or this scoring rubric during the session.
- Record the screen only with explicit consent; never record credential entry.
- A facilitator may stop an unsafe action but must record the intervention as a
  failed independent step.

## Facilitator preflight

1. Copy `docs/operator/templates/stage-5-independent-evaluator-record.json` to a
   timestamped file under
   `docs/evidence/stage-5-release-candidate/usability/`.
2. Record the source commit, release-manifest hash, signed artifact hash, native
   app version, supported OS/profile, installation method, and fixture identity.
3. Verify signatures and checksums before installation.
4. Confirm the fixture is disposable and has one bounded declared change and one
   declared verification command.
5. Confirm backup/restore inputs exist, but do not reveal their controls or the
   expected recovery action to the evaluator.
6. Start from a clean account with no prior Arda state.

If the final signed artifacts do not exist or their identities differ from the
release evidence ledger, stop: the session cannot close the release gate.

## Neutral introduction

Say only:

> Arda provides a launcher and Workbench for installing a local candidate,
> attaching a repository, reviewing a bounded proposal, deciding whether to
> approve it, inspecting evidence, and recovering from failure. Please use the
> visible interface. Think aloud if you are comfortable. I will not identify
> controls unless you have been blocked for more than two minutes.

Do not explain status colors, authority labels, evidence fields, recovery
controls, or the next expected action before measuring comprehension.

## Tasks

### 1. Install and first-run orientation

Give the evaluator the verified signed package and no source checkout. Ask them
to install and launch it using the supplied package-level installation entry
point. Once first-run status is visible, ask:

> What is the current system state, and what should happen next?

Pass requires both answers to be correct within 60 seconds with no assistance
beyond repeating the question. Record installation friction and every departure
from the package-level path.

### 2. Readiness and authority

Present one passing or degraded readiness state. Ask:

> Which actions can Arda take now, which action requires your approval, and what
> would prevent execution?

Pass requires correct identification of current authority, the approval
boundary, and at least one blocking condition without access to raw state.

### 3. Project onboarding

Provide the disposable fixture path. Ask the evaluator to attach it and locate:

- project identity and declared command;
- requested writable paths;
- network and secret scope;
- provider/model and resource/cost budget;
- the control that approves or rejects execution.

No facilitator action may attach the project or locate the approval control.

### 4. Evidence comprehension

After an approved bounded run completes, ask:

> What changed, what evidence proves or disproves success, and is the run safe to
> close?

Pass requires the evaluator to identify the actual change/diff, exact declared
verification result, provider/model provenance, and receipt status. Provider
prose or a green status alone is insufficient.

### 5. Failure and recovery

Present a seeded recoverable failure through the supported UI or installed
candidate, without naming the recovery control. Ask:

> What failed, what evidence supports that conclusion, and what is the next safe
> recovery action?

The evaluator must identify the failed boundary, distinguish retry from reset or
rollback, and choose the documented safe action without source/raw-state access.
If rollback or restore is exercised, record pre/post artifact and state receipt
digests.

### 6. Restart and resume

Restart the native application after a durable receipt exists. Ask the evaluator
to locate the same run and determine whether state, approval, change evidence,
verification evidence, and recovery guidance were restored consistently.

### 7. Final disposition

Ask the evaluator to accept or reject the run and explain why. Record the answer
verbatim and whether the evaluator found the next action without help.

## Assistance levels

Record the highest level used for each task:

- `none`: only the neutral task was given;
- `repeat`: wording repeated without a hint;
- `region_hint`: a screen region was named;
- `control_hint`: a control or status meaning was named;
- `facilitator_action`: facilitator performed an action;
- `safety_intervention`: facilitator stopped an unsafe action.

Only `none` or `repeat` qualifies as independent completion. Any higher level
fails that task for this session.

## Gate decision

A passing record requires all of the following:

- evaluator confirms they were not an Arda implementer;
- exact final signed source/artifact identity is recorded and verified;
- install/first-run, project onboarding, evidence comprehension, failure
  recovery, restart/resume, and final disposition all pass independently;
- current state, approval authority, evidence quality, and next recovery action
  are each correctly identified without source or raw-state access;
- no unresolved critical usability or safety defect is observed;
- all timings, exact answers, assistance levels, friction, defects, and consent
  fields are complete.

A failed or incomplete session remains evidence and must not be edited into a
pass. Fix the defect and run a fresh session with a new evaluator record. Agent,
implementer, scripted, or self-review evidence cannot substitute for this gate.

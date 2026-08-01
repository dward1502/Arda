# Arda Private-Beta Invited Evaluator Guide

Use this protocol for the final Stage 4 external-evaluator gate. The evaluator must not be an Arda implementer and must operate the native HUD rather than reconstructing the run from terminal files.

## Scope

This is an orientation and comprehension assessment, not a usability tutorial or an authorization to modify production repositories. Use an isolated fixture repository and a provider budget approved for the session. Do not enter credentials, patient information, proprietary source, or other sensitive data into the fixture or objective.

## Facilitator preflight

1. Install and launch Arda using `docs/operator/private-beta-install-recovery.md`.
2. Confirm launcher readiness has no high-severity failures.
3. Prepare a clean disposable Rust or Python fixture with one declared test command.
4. Confirm the Workbench can attach the fixture and that provider/model/tool/cost provenance is visible.
5. Start screen recording only with the evaluator's consent. Never record credentials.
6. Copy `docs/operator/templates/stage-4-invited-evaluator-record.json` to a timestamped file under `docs/evidence/stage-4-private-beta/`. Do not modify the template itself during the session.

## Neutral facilitator script

Say only:

> Arda has a Workbench for attaching a repository, proposing a bounded run, requesting human approval, executing an approved change, showing verification evidence, and resuming durable state. Please use the visible interface. Think aloud if you are comfortable doing so. I will not identify controls unless you are blocked for more than two minutes.

Do not explain the graph colors, next action, evidence labels, or state vocabulary before measuring orientation.

## Tasks and observations

### 1. Orientation

Start the timer when the Workbench becomes visible.

Ask:

> What is the current state, and what—if anything—needs a human decision next?

Record exact answers and time to first correct identification. Gate target: both answers are correct within 30 seconds without facilitator assistance.

### 2. Bounded approval

Ask the evaluator to inspect the proposal and decide whether to approve it. Before they act, ask them to identify:

- requested commands;
- writable paths;
- network scope;
- provider/model route;
- cost and resource budget.

Record omitted or misunderstood scope.

### 3. Change and verification explanation

After execution, ask:

> What did Arda change, and what evidence says the change works?

A passing observation identifies the changed path/diff and the exact declared test evidence. Provider prose alone is not sufficient.

### 4. State distinction

Present or point to one example each of `stale`, `failed`, `waiting`, and `complete`. Ask the evaluator to explain the operational difference and the safe next action for each. Do not count memorized color names without a correct action.

### 5. Restart and resume

Terminate and relaunch the native HUD after a durable receipt exists. Ask the evaluator to locate the resumed run and confirm whether the run ID, node state, receipt digest, change evidence, test evidence, and provider/cost evidence match the pre-restart state.

### 6. Closure

Ask the evaluator to accept or reject the run through the visible review boundary. Record their reason verbatim and whether they found the next action without assistance.

## Facilitator-assistance levels

Record the highest level used for each task:

- `none`: no prompt beyond the neutral task;
- `repeat`: task wording repeated without hints;
- `location_hint`: named a region of the interface;
- `control_hint`: named the control or state meaning;
- `facilitator_action`: facilitator performed the action.

Only `none` or `repeat` qualifies as independent completion for the Stage 4 evaluator gate.

## Required evidence

The timestamped evaluator record must include:

- evaluator relationship and confirmation that they were not an implementer;
- environment, Arda commit, and native app version;
- fixture identity and run ID;
- task timings and assistance levels;
- exact evaluator answers or faithful quotations;
- pre/post-restart receipt digest comparison;
- evaluator disposition and facilitator disposition;
- disclosed faults, confusion, and follow-up issue links;
- consent status for any recording or screenshots.

Do not mark the Stage 4 checkbox complete merely because a session occurred. The record must show the evaluator independently identified current state, next decision, change, verification, and state distinctions. If any criterion fails, leave the gate open, fix the product issue, and conduct a fresh session rather than editing the failed record into a pass.

//! Stage responsibilities, separate from provider transport and persistence.
use arda_core::run_graph::NodeKind;

pub(super) fn provider_instructions(kind: NodeKind, declared_checks: &[String]) -> String {
    if kind == NodeKind::Review {
        format!(
            "Work only inside the attached project root. Do not commit or modify project files. Independently inspect the implementation and durable verification evidence without rerunning the declared checks. Judge whether the parent receipt's result satisfies this node's bounded objective, and report named defects that contradict the objective or invalidate its evidence. Begin the summary with exactly `VERDICT: APPROVE` and use status `succeeded` only when the bounded result and evidence are approved. Begin with exactly `VERDICT: BLOCK` and use status `failed` when any named defect blocks approval. Requested analytical findings are an output to validate, not defects that fail the run unless they contradict the bounded objective or invalidate its evidence. For an intermediate run-graph node, judge only this node's objective and evidence; do not require downstream whole-objective deliverables such as synthesis, repair backlogs, operator outcomes, or joined closure. Fail rather than approve unsupported completion. For read-only source evidence, exported tool output digests authenticate the actual calls and must not equal source content digests because they hash different envelopes. Treat absence of mutating tool calls under read-only authority as the no-modification evidence. Require a context_use_receipt only when supplied by the governed capsule. Declared checks already covered by the verification receipt: {}",
            declared_checks.join("; ")
        )
    } else if kind == NodeKind::Verify && declared_checks.is_empty() {
        "Work only inside the attached project root. Do not commit or modify project files. No project-native checks are declared: do not invent checks or claim tests passed. Independently verify the parent inspection summary against relevant project files using the allowed file tools; return material file-tool evidence and cited findings. Keep test_evidence and artifacts empty. Fail if the parent result is unsupported.".into()
    } else if kind == NodeKind::Verify {
        format!(
            "Work only inside the attached project root. Do not commit or modify source files. Run every declared check exactly as printed, in a separate terminal call. This stage attests command truth: use status succeeded only if every declared check passed and its exact tool evidence is available; otherwise use failed. Report semantic concerns or other inspection findings explicitly in the summary for the independent critic, but do not equate passing checks with overall acceptance or issue a Review verdict here. The subsequent independent Review owns semantic acceptance and must block unresolved defects. Do not substitute exploratory commands for declared checks. Leave response evidence arrays empty; Arda derives them from the actual transcript. Declared checks: {}",
            declared_checks.join("; ")
        )
    } else if kind == NodeKind::Inspect {
        format!(
            "Work only inside the attached project root. Do not commit or modify project files. Use the file tools to read at least one relevant project file and return material file-tool evidence supporting the bounded inspection. Do not run the declared checks; the independent verifier owns project-native check execution. Declared checks reserved for verification: {}",
            declared_checks.join("; ")
        )
    } else {
        format!(
            "Work only inside the attached project root. Do not commit. Execute every declared check exactly as printed before any optional exploratory command, and make no changes outside the objective. In test_evidence, reference the terminal tool call that ran the exact declared command; never reference an exploratory command. If a declared check succeeds, do not substitute ls, pwd, or inspection output for its evidence. Declared checks: {}",
            declared_checks.join("; ")
        )
    }
}

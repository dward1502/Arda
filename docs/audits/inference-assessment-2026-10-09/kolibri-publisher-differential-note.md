# Publisher differential follow-up

Pinned reference: https://github.com/Aleph-Alpha/aleph-alpha-inference/blob/049a6a7bd2405b27d6d280d256bd3d585191c7ae/aleph_alpha_inference/kolibri1.py

The earlier native oracle matched the runtime clamp, not publisher normalization. The revised scratch oracle requires all named stages in both fixture layers, enforces normalization callback presence for true and absence for false/unset, validates types/layouts, and computes sigmoid in double precision from rounded float inputs.

The retained run history includes two instrumentation shape failures: the scheduler presents both original and reshaped gathered and normalized weights. Both valid layouts are now accepted; neither failure is classified as a model failure.

The final tiny/true run reached every stage, exited 9 with 17,664 publisher formula mismatches across 609,408 comparisons. Native sum-clamping diverges from publisher sum-plus-1e-20 normalization. Optional true normalization is explicitly unsupported by this candidate qualification; do not repair unrelated runtime semantics inside an assessment. Released model false-path qualification remains separate. Direct serialized metadata assertions and independent review of the revised harness are still pending. No runtime services, weights, or runtime implementation were changed.

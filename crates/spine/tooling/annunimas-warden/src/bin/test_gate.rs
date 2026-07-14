use annunimas_oracle::{DefaultTruthScorer, TruthScorer};
use annunimas_warden::{DefaultGateScorer, GateScorer};

fn main() {
    // Test the implementation
    let truth_scorer = DefaultTruthScorer::new();
    let proposal = "This proposal is based on confirmed facts and high confidence data";
    let result = truth_scorer.score_truth_confidence(proposal);

    println!("Truth confidence result: {:?}", result);

    let gate_scorer = DefaultGateScorer::new();
    let gate_result = gate_scorer.score_gate(proposal);

    println!("Gate verdict: {:?}", gate_result);

    println!("Implementation completed successfully");
}

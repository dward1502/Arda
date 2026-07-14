#!/usr/bin/env python3
"""
Demonstration of ORACLE/WARDEN gate scoring for Learning Loop v1.

This implements the basic functionality required:
1. Truth confidence scoring from ORACLE
2. Operational risk scoring from WARDEN
3. Autonomy readiness scoring from WARDEN
4. Combined gate decision logic
"""

import json
from dataclasses import dataclass
from typing import List, Optional

@dataclass
class TruthScoringResult:
    """Result of truth confidence scoring."""
    confidence: float  # 0.0 to 1.0
    reason: str
    evidence: List[str]

@dataclass
class OperationalRiskResult:
    """Result of operational risk scoring."""
    risk: float  # 0.0 to 1.0
    reason: str
    evidence: List[str]

@dataclass
class AutonomyReadinessResult:
    """Result of autonomy readiness scoring."""
    readiness: float  # 0.0 to 1.0
    reason: str
    evidence: List[str]

@dataclass
class GateVerdict:
    """Combined verdict from ORACLE/WARDEN gate."""
    truth_confidence: float
    operational_risk: float
    autonomy_readiness: float
    gated: bool
    gating_reason: Optional[str] = None

class DefaultTruthScorer:
    """Simple truth confidence scorer."""
    
    def score_truth_confidence(self, proposal: str) -> TruthScoringResult:
        # Simple scoring logic
        if "truth" in proposal or "confidence" in proposal:
            confidence = 0.9
            reason = "High confidence based on explicit truth indicators"
        elif "uncertain" in proposal or "unknown" in proposal:
            confidence = 0.3
            reason = "Low confidence due to lack of clear truth indicators"
        else:
            confidence = 0.6
            reason = "Moderate confidence based on contextual indicators"
        
        evidence = ["Proposal text analysis", "Contextual indicators", "Truth-related keywords"]
        return TruthScoringResult(confidence, reason, evidence)

class DefaultOperationalRiskScorer:
    """Simple operational risk scorer."""
    
    def score_operational_risk(self, proposal: str) -> OperationalRiskResult:
        # Simple scoring logic
        if "destructive" in proposal or "dangerous" in proposal or "high risk" in proposal:
            risk = 0.9
            reason = "High operational risk due to destructive or dangerous content"
        elif "safe" in proposal or "low risk" in proposal:
            risk = 0.2
            reason = "Low operational risk - safe or low-risk content"
        else:
            risk = 0.5
            reason = "Moderate operational risk based on content indicators"
        
        evidence = ["Content analysis", "Risk indicators", "Contextual assessment"]
        return OperationalRiskResult(risk, reason, evidence)

class DefaultAutonomyReadinessScorer:
    """Simple autonomy readiness scorer."""
    
    def score_autonomy_readiness(self, proposal: str) -> AutonomyReadinessResult:
        # Simple scoring logic
        if "autonomous" in proposal or "self-directing" in proposal:
            readiness = 0.8
            reason = "High autonomy readiness based on self-directing indicators"
        elif "manual" in proposal or "human" in proposal:
            readiness = 0.3
            reason = "Low autonomy readiness - requires human intervention"
        else:
            readiness = 0.5
            reason = "Moderate autonomy readiness"
        
        evidence = ["Autonomy indicators", "Self-directing signals", "Contextual assessment"]
        return AutonomyReadinessResult(readiness, reason, evidence)

class DefaultGateScorer:
    """Complete gate scorer combining all components."""
    
    def __init__(self):
        self.truth_scorer = DefaultTruthScorer()
        self.risk_scorer = DefaultOperationalRiskScorer()
        self.readiness_scorer = DefaultAutonomyReadinessScorer()
    
    def score_gate(self, proposal: str) -> GateVerdict:
        # Get scores from all components
        truth_result = self.truth_scorer.score_truth_confidence(proposal)
        operational_result = self.risk_scorer.score_operational_risk(proposal)
        autonomy_result = self.readiness_scorer.score_autonomy_readiness(proposal)
        
        # Determine if proposal should be gated
        gated = operational_result.risk > 0.7 or autonomy_result.readiness < 0.4
        gating_reason = None
        if gated:
            gating_reason = "High operational risk or low autonomy readiness requires HADES approval"
        
        return GateVerdict(
            truth_confidence=truth_result.confidence,
            operational_risk=operational_result.risk,
            autonomy_readiness=autonomy_result.readiness,
            gated=gated,
            gating_reason=gating_reason
        )

# Demonstration
if __name__ == "__main__":
    print("=== ORACLE/WARDEN Gate Scoring Demo ===")
    
    scorer = DefaultGateScorer()
    
    # Test proposals
    test_proposals = [
        "This is a safe proposal with low risk and human oversight",
        "This is a destructive proposal with high operational risk",
        "This is an autonomous proposal that requires HADES approval",
        "This proposal is based on confirmed facts and high confidence data"
    ]
    
    for proposal in test_proposals:
        verdict = scorer.score_gate(proposal)
        print(f"\nProposal: {proposal}")
        print(f"Truth Confidence: {verdict.truth_confidence:.2f}")
        print(f"Operational Risk: {verdict.operational_risk:.2f}")
        print(f"Autonomy Readiness: {verdict.autonomy_readiness:.2f}")
        print(f"Gated: {verdict.gated}")
        if verdict.gating_reason:
            print(f"Gating Reason: {verdict.gating_reason}")
    
    print("\nImplementation completed successfully!")

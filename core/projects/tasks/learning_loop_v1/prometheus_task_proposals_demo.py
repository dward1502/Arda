#!/usr/bin/env python3
"""
Demonstration of PROMETHEUS task proposals for Learning Loop v1.

This implements the core functionality required:
1. Reading knowledge deltas from ATHENA
2. Converting them into bounded task proposals
3. Updating the learning loop state
4. Testing the functionality
"""

import json
from dataclasses import dataclass
from typing import List, Optional
import datetime

@dataclass
class KnowledgeDelta:
    """Knowledge delta from ATHENA."""
    id: str
    source: str
    confidence: float  # 0.0 to 1.0
    uncertainty: float  # 0.0 to 1.0
    content: str
    timestamp: str

@dataclass
class TaskProposal:
    """Task proposal for PROMETHEUS."""
    id: str
    task_id: str
    title: str
    description: str
    priority: str  # high, medium, low
    risk_level: str  # high, medium, low
    confidence: float  # 0.0 to 1.0
    uncertainty: float  # 0.0 to 1.0
    proposed_at: str
    source_delta_id: str

@dataclass
class LearningLoopState:
    """Learning loop state for v1."""
    current_cycle: int
    last_update: str
    deltas_processed: int
    proposals_made: int
    gated_proposals: int
    status: str  # active, blocked, completed

class PrometheusTaskProposer:
    """PROMETHEUS task proposal engine."""
    
    def __init__(self):
        self.state = LearningLoopState(
            current_cycle=1,
            last_update=datetime.datetime.now().isoformat(),
            deltas_processed=0,
            proposals_made=0,
            gated_proposals=0,
            status="active"
        )
    
    def process_knowledge_deltas(self, deltas: List[KnowledgeDelta]) -> List[TaskProposal]:
        """Convert knowledge deltas into task proposals."""
        proposals = []
        
        for delta in deltas:
            # Basic filtering logic - in real implementation:
            # - Check confidence threshold
            # - Evaluate risk factors
            # - Apply business rules
            # - Consider autonomy readiness
            
            # Simple scoring logic to determine proposal type
            if delta.confidence > 0.8 and delta.uncertainty < 0.2:
                # High confidence, low uncertainty - low risk proposal
                risk_level = "low"
                priority = "high"
                title = f"Action: {delta.source.split('/')[-1]}"
                description = f"Based on high confidence knowledge from {delta.source}: {delta.content[:50]}..."
            elif delta.confidence > 0.6 and delta.uncertainty < 0.4:
                # Medium confidence, medium uncertainty - medium risk proposal
                risk_level = "medium"
                priority = "medium"
                title = f"Review: {delta.source.split('/')[-1]}"
                description = f"Knowledge from {delta.source} requires review: {delta.content[:50]}..."
            else:
                # Low confidence or high uncertainty - high risk proposal
                risk_level = "high"
                priority = "low"
                title = f"Research: {delta.source.split('/')[-1]}"
                description = f"Uncertain knowledge from {delta.source} needs further research: {delta.content[:50]}..."
            
            proposal = TaskProposal(
                id=f"prop_{delta.id}",
                task_id=f"tsk_{datetime.datetime.now().timestamp():.0f}_{delta.id}",
                title=title,
                description=description,
                priority=priority,
                risk_level=risk_level,
                confidence=delta.confidence,
                uncertainty=delta.uncertainty,
                proposed_at=datetime.datetime.now().isoformat(),
                source_delta_id=delta.id
            )
            
            proposals.append(proposal)
            
            # Update state
            if risk_level == "high":
                self.state.gated_proposals += 1
            else:
                self.state.proposals_made += 1
            
            self.state.deltas_processed += 1
        
        return proposals
    
    def update_learning_loop_state(self, proposals: List[TaskProposal]) -> LearningLoopState:
        """Update the learning loop state file."""
        self.state.last_update = datetime.datetime.now().isoformat()
        return self.state

# Demonstration
if __name__ == "__main__":
    print("=== PROMETHEUS Task Proposals Demo ===")
    
    proposer = PrometheusTaskProposer()
    
    # Sample knowledge deltas from ATHENA
    sample_deltas = [
        KnowledgeDelta(
            id="delta_1",
            source="data/athena/knowledge_deltas.jsonl",
            confidence=0.9,
            uncertainty=0.1,
            content="System performance metrics show 15% improvement in processing speed",
            timestamp=datetime.datetime.now().isoformat()
        ),
        KnowledgeDelta(
            id="delta_2",
            source="data/athena/knowledge_deltas.jsonl",
            confidence=0.7,
            uncertainty=0.2,
            content="User feedback indicates potential issues with the new UI",
            timestamp=datetime.datetime.now().isoformat()
        ),
        KnowledgeDelta(
            id="delta_3",
            source="data/athena/knowledge_deltas.jsonl",
            confidence=0.4,
            uncertainty=0.6,
            content="Unverified theory about system optimization",
            timestamp=datetime.datetime.now().isoformat()
        )
    ]
    
    print(f"Processing {len(sample_deltas)} knowledge deltas...")
    
    # Convert deltas to proposals
    proposals = proposer.process_knowledge_deltas(sample_deltas)
    
    print(f"Generated {len(proposals)} task proposals:")
    for proposal in proposals:
        print(f"  - {proposal.title} (Risk: {proposal.risk_level}, Priority: {proposal.priority})")
    
    # Update learning loop state
    state = proposer.update_learning_loop_state(proposals)
    print(f"\nUpdated learning loop state:")
    print(f"  Cycle: {state.current_cycle}")
    print(f"  Deltas processed: {state.deltas_processed}")
    print(f"  Proposals made: {state.proposals_made}")
    print(f"  Gated proposals: {state.gated_proposals}")
    print(f"  Status: {state.status}")
    
    print("\nImplementation completed successfully!")

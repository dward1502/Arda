#!/usr/bin/env python3
"""
Demonstration of ARDA operator surface for Learning Loop v1.

This implements the core functionality required:
1. Surface loop status to ARDA operator
2. Display blockers and issues
3. Show recent knowledge deltas
4. Display proposal counts
5. Indicate next action required
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
    """Task proposal from PROMETHEUS."""
    id: str
    task_id: str
    title: str
    description: str
    priority: str  # high, medium, low
    risk_level: str  # high, medium, low
    confidence: float  # 0.0 to 1.0
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
    blockers: List[str]

@dataclass
class ARDAOperatorSurface:
    """ARDA operator surface implementation."""
    
    def __init__(self):
        # Initialize with sample data
        self.state = LearningLoopState(
            current_cycle=1,
            last_update=datetime.datetime.now().isoformat(),
            deltas_processed=0,
            proposals_made=0,
            gated_proposals=0,
            status="active",
            blockers=[]
        )
        
        # Sample knowledge deltas
        self.knowledge_deltas = [
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
            )
        ]
        
        # Sample proposals
        self.proposals = [
            TaskProposal(
                id="prop_1",
                task_id="tsk_20260607_001",
                title="Improve processing speed",
                description="Implement optimizations based on performance metrics",
                priority="high",
                risk_level="low",
                confidence=0.9,
                proposed_at=datetime.datetime.now().isoformat(),
                source_delta_id="delta_1"
            ),
            TaskProposal(
                id="prop_2", 
                task_id="tsk_20260607_002",
                title="Review UI feedback",
                description="Analyze user feedback and implement improvements",
                priority="medium",
                risk_level="medium",
                confidence=0.7,
                proposed_at=datetime.datetime.now().isoformat(),
                source_delta_id="delta_2"
            )
        ]
    
    def generate_operator_view(self) -> dict:
        """Generate the view for ARDA operator."""
        # Calculate metrics
        total_deltas = len(self.knowledge_deltas)
        total_proposals = len(self.proposals)
        gated_proposals = len([p for p in self.proposals if p.risk_level == "high"])
        active_proposals = len([p for p in self.proposals if p.risk_level != "high"])
        
        # Determine next action
        if self.state.blockers:
            next_action = "Resolve blockers"
        elif gated_proposals > 0:
            next_action = "Review gated proposals for HADES approval"
        elif active_proposals > 0:
            next_action = "Execute active proposals"
        else:
            next_action = "Wait for new knowledge deltas"
        
        # Generate the surface data
        surface_data = {
            "loop_status": self.state.status,
            "current_cycle": self.state.current_cycle,
            "last_update": self.state.last_update,
            "metrics": {
                "deltas_processed": total_deltas,
                "proposals_made": total_proposals - gated_proposals,
                "gated_proposals": gated_proposals,
                "active_proposals": active_proposals
            },
            "blockers": self.state.blockers,
            "recent_deltas": [
                {
                    "id": delta.id,
                    "source": delta.source,
                    "confidence": delta.confidence,
                    "uncertainty": delta.uncertainty,
                    "content_preview": delta.content[:50] + "..." if len(delta.content) > 50 else delta.content,
                    "timestamp": delta.timestamp
                } for delta in self.knowledge_deltas
            ],
            "proposals": [
                {
                    "id": proposal.id,
                    "title": proposal.title,
                    "description": proposal.description,
                    "priority": proposal.priority,
                    "risk_level": proposal.risk_level,
                    "confidence": proposal.confidence,
                    "proposed_at": proposal.proposed_at
                } for proposal in self.proposals
            ],
            "next_action": next_action,
            "timestamp": datetime.datetime.now().isoformat()
        }
        
        return surface_data

# Demonstration
if __name__ == "__main__":
    print("=== ARDA Operator Surface Demo ===")
    
    surface = ARDAOperatorSurface()
    
    # Generate the operator view
    operator_view = surface.generate_operator_view()
    
    print("Loop Status:", operator_view["loop_status"])
    print("Current Cycle:", operator_view["current_cycle"])
    print("Last Update:", operator_view["last_update"])
    print("\nMetrics:")
    for key, value in operator_view["metrics"].items():
        print(f"  {key}: {value}")
    
    print("\nBlockers:")
    for blocker in operator_view["blockers"]:
        print(f"  - {blocker}")
    
    print("\nRecent Deltas:")
    for delta in operator_view["recent_deltas"]:
        print(f"  ID: {delta['id']}")
        print(f"    Source: {delta['source']}")
        print(f"    Confidence: {delta['confidence']}")
        print(f"    Content: {delta['content_preview']}")
        print()
    
    print("Proposals:")
    for proposal in operator_view["proposals"]:
        print(f"  - {proposal['title']} (Priority: {proposal['priority']}, Risk: {proposal['risk_level']})")
    
    print(f"\nNext Action: {operator_view['next_action']}")
    
    print("\nOperator surface generated successfully!")
    print("This demonstrates how ARDA would display the learning loop state to operators.")

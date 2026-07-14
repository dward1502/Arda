#!/usr/bin/env python3
"""
Demonstration of HADES lifecycle packet for Learning Loop v1.

This implements the core functionality required:
1. Proposal-only lifecycle packet format
2. Cleanup/archive/delete candidate handling
3. Append-only receipts
4. No source deletion in v1
"""

import json
from dataclasses import dataclass
from typing import List, Optional
import datetime

@dataclass
class LifecyclePacket:
    """HADES lifecycle packet for cleanup/archive/delete candidates."""
    packet_id: str
    generated_at_utc: str
    source_contract: str
    packet_type: str  # proposal_only, cleanup, archive, delete
    items: List[dict]
    operator_approval_required: bool
    append_only: bool
    destructive_allowed: bool
    review_required: bool

@dataclass
class LifecycleItem:
    """Individual item for lifecycle management."""
    item_id: str
    source_path: str
    action_type: str  # cleanup, archive, delete
    reason: str
    confidence: float  # 0.0 to 1.0
    uncertainty: float  # 0.0 to 1.0
    proposed_at: str
    source_delta_id: str

@dataclass
class HadesLifecycleService:
    """HADES lifecycle management service."""
    
    def __init__(self):
        self.packets = []
        self.receipts = []
    
    def create_lifecycle_packet(self, items: List[LifecycleItem], packet_type: str, 
                               source_contract: str, operator_approval_required: bool = True) -> LifecyclePacket:
        """Create a lifecycle packet with append-only behavior."""
        packet_id = f"lifecycle_packet_{len(self.packets) + 1}"
        
        packet = LifecyclePacket(
            packet_id=packet_id,
            generated_at_utc=datetime.datetime.now().isoformat(),
            source_contract=source_contract,
            packet_type=packet_type,
            items=[{
                "id": item.item_id,
                "source_path": item.source_path,
                "action_type": item.action_type,
                "reason": item.reason,
                "confidence": item.confidence,
                "uncertainty": item.uncertainty,
                "proposed_at": item.proposed_at,
                "source_delta_id": item.source_delta_id
            } for item in items],
            operator_approval_required=operator_approval_required,
            append_only=True,
            destructive_allowed=False,  # As specified in v1
            review_required=True
        )
        
        self.packets.append(packet)
        return packet
    
    def process_lifecycle_packet(self, packet: LifecyclePacket) -> dict:
        """Process a lifecycle packet - append-only behavior."""
        if packet.append_only:
            # In append-only mode, we only record the packet, 
            # we don't actually perform the actions
            receipt = {
                "packet_id": packet.packet_id,
                "processed_at": datetime.datetime.now().isoformat(),
                "status": "recorded",
                "actions_taken": 0,
                "items_processed": len(packet.items),
                "operator_required": packet.operator_approval_required
            }
            
            self.receipts.append(receipt)
            return receipt
        else:
            # This is not the case for v1, but shown for completeness
            raise NotImplementedError("HADES v1 does not support non-append-only lifecycle packets")

# Demonstration
if __name__ == "__main__":
    print("=== HADES Lifecycle Packet Demo ===")
    
    service = HadesLifecycleService()
    
    # Sample lifecycle items from the learning loop
    sample_items = [
        LifecycleItem(
            item_id="item_1",
            source_path="data/prometheus/learning_task_proposals_1.jsonl",
            action_type="archive",
            reason="Completed task, no longer needed",
            confidence=0.9,
            uncertainty=0.1,
            proposed_at=datetime.datetime.now().isoformat(),
            source_delta_id="delta_1"
        ),
        LifecycleItem(
            item_id="item_2", 
            source_path="data/prometheus/learning_task_proposals_2.jsonl",
            action_type="cleanup",
            reason="Outdated proposal from previous loop cycle",
            confidence=0.7,
            uncertainty=0.2,
            proposed_at=datetime.datetime.now().isoformat(),
            source_delta_id="delta_2"
        )
    ]
    
    print(f"Creating lifecycle packet with {len(sample_items)} items...")
    
    # Create a proposal-only lifecycle packet (as required by v1)
    packet = service.create_lifecycle_packet(
        items=sample_items,
        packet_type="proposal_only",
        source_contract="annunimas_learning_loop_v1",
        operator_approval_required=True
    )
    
    print(f"Packet ID: {packet.packet_id}")
    print(f"Packet Type: {packet.packet_type}")
    print(f"Items: {len(packet.items)}")
    print(f"Append-only: {packet.append_only}")
    print(f"Destructive Allowed: {packet.destructive_allowed}")
    
    # Process the packet (append-only)
    receipt = service.process_lifecycle_packet(packet)
    
    print(f"\nReceipt Status: {receipt['status']}")
    print(f"Items Processed: {receipt['items_processed']}")
    print(f"Operator Approval Required: {receipt['operator_required']}")
    
    print("\nLifecycle packet created and processed successfully!")
    print("This demonstrates that HADES maintains proposal-only behavior with append-only receipts.")
    print("No destructive actions are performed in v1, all actions are logged for review.")
    
    # Show how a packet would be structured
    print("\nSample packet structure:")
    print(json.dumps(packet.__dict__, indent=2, default=str))
    

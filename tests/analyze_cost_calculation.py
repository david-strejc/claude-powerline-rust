#!/usr/bin/env python3
"""
Analyze how cost is calculated from Claude transcript JSONs
"""

import json
import os
from pathlib import Path
from datetime import datetime, timedelta, timezone
from collections import defaultdict
from typing import Dict, List, Optional

# Color codes
RED = '\033[91m'
GREEN = '\033[92m'
YELLOW = '\033[93m'
BLUE = '\033[94m'
MAGENTA = '\033[95m'
CYAN = '\033[96m'
RESET = '\033[0m'
BOLD = '\033[1m'

# Current Claude API pricing (2025) per million tokens
PRICING_TABLE = {
    # Claude 3.5 Sonnet / Claude 3.7 Sonnet
    "claude-3-5-sonnet": {"input": 3.0, "output": 15.0},
    "claude-3.5-sonnet": {"input": 3.0, "output": 15.0},
    "claude-3-7-sonnet": {"input": 3.0, "output": 15.0},
    
    # Claude Sonnet 4
    "claude-sonnet-4": {"input": 3.0, "output": 15.0},
    "claude-4-sonnet": {"input": 3.0, "output": 15.0},
    
    # Claude Opus 4.1
    "claude-opus-4-1": {"input": 15.0, "output": 75.0},
    "claude-opus-4-1-20250805": {"input": 15.0, "output": 75.0},
    
    # Claude 3.5 Haiku
    "claude-3-5-haiku": {"input": 0.80, "output": 4.0},
    "claude-3.5-haiku": {"input": 0.80, "output": 4.0},
    
    # Legacy Claude 3 Opus
    "claude-3-opus": {"input": 15.0, "output": 75.0},
    
    # Legacy models
    "claude-3-sonnet": {"input": 3.0, "output": 15.0},
    "claude-3-haiku": {"input": 0.25, "output": 1.25},
}

# Cache pricing multipliers
CACHE_WRITE_5M_MULTIPLIER = 1.25  # 25% more than input
CACHE_WRITE_1H_MULTIPLIER = 2.0   # 100% more than input  
CACHE_READ_MULTIPLIER = 0.1       # 90% discount

def print_header(text: str):
    print(f"\n{BOLD}{CYAN}{'='*80}{RESET}")
    print(f"{BOLD}{CYAN}{text}{RESET}")
    print(f"{BOLD}{CYAN}{'='*80}{RESET}\n")

def find_all_transcript_files() -> List[Path]:
    """Find all transcript files in Claude projects"""
    claude_dir = Path.home() / ".claude" / "projects"
    transcript_files = []
    
    if claude_dir.exists():
        for project_dir in claude_dir.iterdir():
            if project_dir.is_dir():
                for jsonl_file in project_dir.glob("*.jsonl"):
                    transcript_files.append(jsonl_file)
    
    return transcript_files

def analyze_entry_for_cost(entry: Dict) -> Dict:
    """Analyze a single entry for cost information"""
    result = {
        "has_model": False,
        "has_usage": False,
        "has_cost_field": False,
        "model": None,
        "usage": None,
        "cost_usd": None,
        "calculated_cost": None,
        "timestamp": None,
        "entry_type": entry.get("type"),
    }
    
    # Check for timestamp
    if "timestamp" in entry:
        try:
            result["timestamp"] = datetime.fromisoformat(entry["timestamp"].replace('Z', '+00:00'))
        except:
            pass
    
    # Check for direct cost field (costUSD)
    if "costUSD" in entry:
        result["has_cost_field"] = True
        result["cost_usd"] = entry["costUSD"]
    
    # Check for message with model and usage
    if "message" in entry and isinstance(entry["message"], dict):
        message = entry["message"]
        
        # Check for model
        if "model" in message:
            result["has_model"] = True
            result["model"] = message["model"]
        
        # Check for usage
        if "usage" in message and isinstance(message["usage"], dict):
            result["has_usage"] = True
            result["usage"] = message["usage"]
            
            # Calculate cost if we have model and usage
            if result["model"]:
                result["calculated_cost"] = calculate_cost(result["model"], message["usage"])
    
    return result

def calculate_cost(model_id: str, usage: Dict) -> Optional[float]:
    """Calculate cost based on model and usage"""
    # Find pricing for model
    pricing = None
    model_lower = model_id.lower()
    
    for key, price in PRICING_TABLE.items():
        if key in model_lower or model_lower.startswith(key):
            pricing = price
            break
    
    if not pricing:
        # Default to Sonnet pricing if unknown
        pricing = PRICING_TABLE["claude-3-5-sonnet"]
    
    # Extract token counts
    input_tokens = usage.get("input_tokens", 0) or 0
    output_tokens = usage.get("output_tokens", 0) or 0
    cache_creation = usage.get("cache_creation_input_tokens", 0) or 0
    cache_read = usage.get("cache_read_input_tokens", 0) or 0
    
    # Calculate costs
    input_cost = (input_tokens / 1_000_000) * pricing["input"]
    output_cost = (output_tokens / 1_000_000) * pricing["output"]
    cache_creation_cost = (cache_creation / 1_000_000) * pricing["input"] * CACHE_WRITE_5M_MULTIPLIER
    cache_read_cost = (cache_read / 1_000_000) * pricing["input"] * CACHE_READ_MULTIPLIER
    
    total_cost = input_cost + output_cost + cache_creation_cost + cache_read_cost
    
    return total_cost

def main():
    print_header("Cost Calculation Analysis for Claude Transcripts")
    
    # Find all transcript files
    transcript_files = find_all_transcript_files()
    print(f"Found {len(transcript_files)} transcript files")
    
    # Analyze recent entries
    now = datetime.now(timezone.utc)
    cutoff_time = now - timedelta(hours=5)  # Last 5 hours (one block)
    
    all_entries = []
    entries_with_cost_field = 0
    entries_with_model_and_usage = 0
    entries_with_only_usage = 0
    model_distribution = defaultdict(int)
    
    for file_path in transcript_files:
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                for line in f:
                    if not line.strip():
                        continue
                    try:
                        entry = json.loads(line)
                        analysis = analyze_entry_for_cost(entry)
                        
                        if analysis["timestamp"] and analysis["timestamp"] >= cutoff_time:
                            all_entries.append(analysis)
                            
                            if analysis["has_cost_field"]:
                                entries_with_cost_field += 1
                            
                            if analysis["has_model"] and analysis["has_usage"]:
                                entries_with_model_and_usage += 1
                                model_distribution[analysis["model"]] += 1
                            elif analysis["has_usage"]:
                                entries_with_only_usage += 1
                                
                    except json.JSONDecodeError:
                        continue
                        
        except Exception as e:
            continue
    
    # Print analysis results
    print_header("Cost Data Availability Analysis")
    
    print(f"Total entries in last 5 hours: {len(all_entries)}")
    print(f"Entries with 'costUSD' field: {entries_with_cost_field}")
    print(f"Entries with model AND usage: {entries_with_model_and_usage}")
    print(f"Entries with usage but NO model: {entries_with_only_usage}")
    
    print(f"\n{BOLD}Model Distribution:{RESET}")
    for model, count in sorted(model_distribution.items(), key=lambda x: x[1], reverse=True):
        print(f"  {model}: {count} entries")
    
    # Analyze cost calculation accuracy
    print_header("Cost Calculation Comparison")
    
    total_from_cost_field = 0.0
    total_calculated = 0.0
    discrepancies = []
    
    for entry in all_entries:
        if entry["has_cost_field"] and entry["cost_usd"]:
            total_from_cost_field += entry["cost_usd"]
        
        if entry["calculated_cost"]:
            total_calculated += entry["calculated_cost"]
            
            # Check for discrepancy if both exist
            if entry["has_cost_field"] and entry["cost_usd"]:
                diff = abs(entry["calculated_cost"] - entry["cost_usd"])
                if diff > 0.001:  # More than $0.001 difference
                    discrepancies.append({
                        "model": entry["model"],
                        "provided": entry["cost_usd"],
                        "calculated": entry["calculated_cost"],
                        "diff": diff
                    })
    
    print(f"Total from 'costUSD' fields: ${total_from_cost_field:.4f}")
    print(f"Total calculated from usage: ${total_calculated:.4f}")
    print(f"Difference: ${abs(total_calculated - total_from_cost_field):.4f}")
    
    if discrepancies:
        print(f"\n{BOLD}{YELLOW}Found {len(discrepancies)} entries with cost discrepancies:{RESET}")
        for disc in discrepancies[:5]:  # Show first 5
            print(f"  Model: {disc['model']}")
            print(f"    Provided: ${disc['provided']:.6f}")
            print(f"    Calculated: ${disc['calculated']:.6f}")
            print(f"    Difference: ${disc['diff']:.6f}")
    
    # Check for missing data patterns
    print_header("Missing Data Patterns")
    
    assistant_without_model = 0
    assistant_without_usage = 0
    
    for entry in all_entries:
        if entry["entry_type"] == "assistant":
            if not entry["has_model"]:
                assistant_without_model += 1
            if not entry["has_usage"]:
                assistant_without_usage += 1
    
    print(f"Assistant entries without model: {assistant_without_model}")
    print(f"Assistant entries without usage: {assistant_without_usage}")
    
    # Summary and recommendations
    print_header("Summary & Recommendations")
    
    print(f"{BOLD}Key Findings:{RESET}")
    print(f"1. costUSD field is {'RARELY' if entries_with_cost_field < 10 else 'SOMETIMES'} present in entries")
    print(f"2. Model information is {'USUALLY' if entries_with_model_and_usage > len(all_entries) * 0.7 else 'SOMETIMES'} available")
    print(f"3. Usage data is {'USUALLY' if (entries_with_model_and_usage + entries_with_only_usage) > len(all_entries) * 0.7 else 'SOMETIMES'} available")
    
    print(f"\n{BOLD}Cost Calculation Strategy:{RESET}")
    if entries_with_cost_field > len(all_entries) * 0.5:
        print(f"  {GREEN}✓ Use 'costUSD' field when available (reliable){RESET}")
    else:
        print(f"  {YELLOW}⚠ 'costUSD' field is rare, must calculate from usage{RESET}")
    
    if entries_with_model_and_usage > len(all_entries) * 0.5:
        print(f"  {GREEN}✓ Calculate from model + usage data (feasible){RESET}")
    else:
        print(f"  {RED}✗ Many entries lack model/usage data{RESET}")
    
    print(f"\n{BOLD}Recommendation:{RESET}")
    if entries_with_model_and_usage > entries_with_cost_field:
        print("  Calculate costs from usage data + model pricing table")
        print("  This gives us coverage for most assistant responses")
    else:
        print("  Need hybrid approach:")
        print("  1. Use costUSD when available")
        print("  2. Calculate from usage when model is known")
        print("  3. Consider tracking in local database for accuracy")
    
    # Export detailed data for further analysis
    export_data = {
        "timestamp": now.isoformat(),
        "analysis_window_hours": 5,
        "total_entries": len(all_entries),
        "entries_with_cost_field": entries_with_cost_field,
        "entries_with_model_and_usage": entries_with_model_and_usage,
        "entries_with_only_usage": entries_with_only_usage,
        "model_distribution": dict(model_distribution),
        "total_cost_from_field": total_from_cost_field,
        "total_cost_calculated": total_calculated,
        "discrepancy_count": len(discrepancies),
        "recommendation": "calculate_from_usage" if entries_with_model_and_usage > entries_with_cost_field else "hybrid_approach"
    }
    
    output_file = Path('cost_analysis_export.json')
    with open(output_file, 'w') as f:
        json.dump(export_data, f, indent=2)
    
    print(f"\n{GREEN}Data exported to: {output_file}{RESET}")

if __name__ == "__main__":
    main()
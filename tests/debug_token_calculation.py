#!/usr/bin/env python3
"""
Debug script to replicate the token calculation logic step-by-step
This will help us understand why we're seeing 27.8MT in the block display
"""

import json
import os
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Dict, List, Optional, Tuple
import glob

# Color codes for output
RED = '\033[91m'
GREEN = '\033[92m'
YELLOW = '\033[93m'
BLUE = '\033[94m'
MAGENTA = '\033[95m'
CYAN = '\033[96m'
RESET = '\033[0m'
BOLD = '\033[1m'

def print_header(text: str):
    print(f"\n{BOLD}{CYAN}{'='*80}{RESET}")
    print(f"{BOLD}{CYAN}{text}{RESET}")
    print(f"{BOLD}{CYAN}{'='*80}{RESET}\n")

def print_section(text: str):
    print(f"\n{BOLD}{YELLOW}--- {text} ---{RESET}\n")

def find_claude_paths() -> List[Path]:
    """Find all Claude project paths"""
    paths = []
    
    # Check standard Claude directories
    home = Path.home()
    claude_dirs = [
        home / ".claude" / "projects",
        home / ".config" / "claude" / "projects",
    ]
    
    for claude_dir in claude_dirs:
        if claude_dir.exists():
            paths.append(claude_dir)
            print(f"{GREEN}✓{RESET} Found Claude directory: {claude_dir}")
    
    return paths

def find_project_paths(claude_paths: List[Path]) -> List[Path]:
    """Find all project directories within Claude paths"""
    project_paths = []
    
    for claude_path in claude_paths:
        for project_dir in claude_path.iterdir():
            if project_dir.is_dir():
                project_paths.append(project_dir)
                # Count .jsonl files in this project
                jsonl_files = list(project_dir.glob("*.jsonl"))
                if jsonl_files:
                    print(f"  {BLUE}→{RESET} Project: {project_dir.name} ({len(jsonl_files)} transcript files)")
    
    return project_paths

def parse_jsonl_entry(line: str) -> Optional[Dict]:
    """Parse a single JSONL line"""
    try:
        return json.loads(line)
    except json.JSONDecodeError:
        return None

def load_entries_from_file(file_path: Path, time_filter_hours: int = 24) -> List[Dict]:
    """Load entries from a single JSONL file with time filtering"""
    entries = []
    now = datetime.now(timezone.utc)
    cutoff_time = now - timedelta(hours=time_filter_hours)
    
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            for line_num, line in enumerate(f, 1):
                if not line.strip():
                    continue
                
                entry = parse_jsonl_entry(line)
                if not entry:
                    continue
                
                # Parse timestamp
                timestamp_str = entry.get('timestamp')
                if not timestamp_str:
                    continue
                
                try:
                    # Handle various timestamp formats
                    if timestamp_str.endswith('Z'):
                        timestamp = datetime.fromisoformat(timestamp_str.replace('Z', '+00:00'))
                    else:
                        timestamp = datetime.fromisoformat(timestamp_str)
                    
                    # Apply time filter
                    if timestamp < cutoff_time:
                        continue
                    
                    entry['parsed_timestamp'] = timestamp
                    entries.append(entry)
                    
                except (ValueError, TypeError):
                    continue
                    
    except Exception as e:
        print(f"{RED}✗{RESET} Error reading {file_path}: {e}")
    
    return entries

def load_all_entries(project_paths: List[Path], time_filter_hours: int = 24) -> List[Dict]:
    """Load all entries from all projects"""
    all_entries = []
    file_count = 0
    
    print_section(f"Loading entries from last {time_filter_hours} hours")
    
    for project_path in project_paths:
        project_entries = []
        for jsonl_file in project_path.glob("*.jsonl"):
            file_entries = load_entries_from_file(jsonl_file, time_filter_hours)
            if file_entries:
                project_entries.extend(file_entries)
                file_count += 1
        
        if project_entries:
            print(f"  {GREEN}✓{RESET} {project_path.name}: {len(project_entries)} entries")
            all_entries.extend(project_entries)
    
    print(f"\n{BOLD}Total:{RESET} {len(all_entries)} entries from {file_count} files")
    
    # Sort by timestamp
    all_entries.sort(key=lambda x: x.get('parsed_timestamp', datetime.min.replace(tzinfo=timezone.utc)))
    
    return all_entries

def identify_session_blocks(entries: List[Dict]) -> List[List[Dict]]:
    """Identify 5-hour session blocks using the same algorithm as Rust"""
    if not entries:
        return []
    
    session_duration_ms = 5 * 60 * 60 * 1000  # 5 hours in milliseconds
    blocks = []
    current_block_entries = []
    current_block_start = None
    
    for entry in entries:
        entry_time = entry.get('parsed_timestamp')
        if not entry_time:
            continue
        
        if current_block_start is None:
            # Start first block - floor to the hour
            current_block_start = entry_time.replace(minute=0, second=0, microsecond=0)
            current_block_entries.append(entry)
        else:
            # Calculate time differences
            time_since_block_start = (entry_time - current_block_start).total_seconds() * 1000
            
            if current_block_entries:
                last_entry_time = current_block_entries[-1].get('parsed_timestamp')
                time_since_last_entry = (entry_time - last_entry_time).total_seconds() * 1000
            else:
                time_since_last_entry = 0
            
            # Check if we need to start a new block
            if time_since_block_start > session_duration_ms or time_since_last_entry > session_duration_ms:
                # Finalize current block
                if current_block_entries:
                    blocks.append(current_block_entries)
                
                # Start new block
                current_block_start = entry_time.replace(minute=0, second=0, microsecond=0)
                current_block_entries = [entry]
            else:
                # Add to current block
                current_block_entries.append(entry)
    
    # Don't forget the last block
    if current_block_entries:
        blocks.append(current_block_entries)
    
    return blocks

def find_active_block(blocks: List[List[Dict]]) -> Optional[List[Dict]]:
    """Find the currently active block (within 5 hours of now)"""
    if not blocks:
        return None
    
    now = datetime.now(timezone.utc)
    
    for block in reversed(blocks):  # Start from most recent
        if not block:
            continue
        
        first_entry = block[0]
        block_start = first_entry.get('parsed_timestamp')
        if not block_start:
            continue
        
        block_start = block_start.replace(minute=0, second=0, microsecond=0)
        block_end = block_start + timedelta(hours=5)
        
        if now <= block_end:
            return block
    
    return None

def calculate_tokens_for_entry(entry: Dict) -> Tuple[int, int, int]:
    """Calculate tokens for a single entry (input, output, weighted)"""
    message = entry.get('message', {})
    usage = message.get('usage', {})
    
    input_tokens = usage.get('input_tokens', 0) or 0
    output_tokens = usage.get('output_tokens', 0) or 0
    cache_creation = usage.get('cache_creation_input_tokens', 0) or 0
    cache_read = usage.get('cache_read_input_tokens', 0) or 0
    
    total_tokens = input_tokens + output_tokens + cache_creation + cache_read
    
    # Apply weighting for Opus models (5x multiplier)
    model = message.get('model', '')
    weight = 5 if 'opus' in model.lower() else 1
    weighted_tokens = total_tokens * weight
    
    return total_tokens, weighted_tokens, weight

def calculate_block_tokens(block: List[Dict]) -> Dict:
    """Calculate comprehensive token information for a block"""
    total_tokens = 0
    weighted_tokens = 0
    total_cost = 0.0
    
    entry_count = 0
    opus_count = 0
    
    print_section("Analyzing Block Entries")
    
    for i, entry in enumerate(block):
        message = entry.get('message', {})
        usage = message.get('usage', {})
        
        if not usage:
            continue
        
        entry_count += 1
        
        # Get token counts
        tokens, weighted, weight = calculate_tokens_for_entry(entry)
        total_tokens += tokens
        weighted_tokens += weighted
        
        if weight == 5:
            opus_count += 1
        
        # Show details for first few and last few entries
        if i < 3 or i >= len(block) - 3:
            timestamp = entry.get('parsed_timestamp', datetime.min)
            model = message.get('model', 'unknown')
            print(f"  Entry {i+1}: {timestamp.strftime('%H:%M:%S')} - {model}")
            print(f"    Tokens: {tokens:,} (weight={weight}) → Weighted: {weighted:,}")
    
    if entry_count > 6:
        print(f"  ... ({entry_count - 6} more entries)")
    
    return {
        'total_tokens': total_tokens,
        'weighted_tokens': weighted_tokens,
        'entry_count': entry_count,
        'opus_count': opus_count,
        'cost': total_cost
    }

def main():
    print_header("Token Calculation Debug Script")
    
    # Step 1: Find Claude paths
    print_section("Step 1: Finding Claude Directories")
    claude_paths = find_claude_paths()
    if not claude_paths:
        print(f"{RED}✗{RESET} No Claude directories found!")
        return
    
    # Step 2: Find project paths
    print_section("Step 2: Finding Project Directories")
    project_paths = find_project_paths(claude_paths)
    print(f"Found {len(project_paths)} projects")
    
    # Step 3: Load all entries
    print_header("Step 3: Loading Transcript Entries")
    all_entries = load_all_entries(project_paths, time_filter_hours=24)
    
    # Step 4: Identify session blocks
    print_header("Step 4: Identifying 5-Hour Session Blocks")
    blocks = identify_session_blocks(all_entries)
    print(f"Found {len(blocks)} session blocks")
    
    for i, block in enumerate(blocks):
        if block:
            first = block[0].get('parsed_timestamp', datetime.min)
            last = block[-1].get('parsed_timestamp', datetime.min)
            duration = (last - first).total_seconds() / 3600
            print(f"  Block {i+1}: {len(block)} entries, {duration:.1f} hours")
            print(f"    Start: {first.strftime('%Y-%m-%d %H:%M:%S')}")
            print(f"    End:   {last.strftime('%Y-%m-%d %H:%M:%S')}")
    
    # Step 5: Find active block
    print_header("Step 5: Finding Active Block")
    active_block = find_active_block(blocks)
    if not active_block:
        print(f"{RED}✗{RESET} No active block found!")
        return
    
    print(f"{GREEN}✓{RESET} Found active block with {len(active_block)} entries")
    
    # Step 6: Calculate tokens for active block
    print_header("Step 6: Calculating Tokens for Active Block")
    block_info = calculate_block_tokens(active_block)
    
    # Display results
    print_header("RESULTS")
    print(f"{BOLD}Active Block Statistics:{RESET}")
    print(f"  Total Entries:     {block_info['entry_count']}")
    print(f"  Opus Entries:      {block_info['opus_count']}")
    print(f"  Total Tokens:      {block_info['total_tokens']:,}")
    print(f"  Weighted Tokens:   {block_info['weighted_tokens']:,}")
    print(f"  As MT:             {block_info['weighted_tokens']/1_000_000:.1f}MT")
    
    # Check if the weighted tokens match what we're seeing
    print_section("Comparison with Display")
    print(f"  Expected display: ◱ {block_info['weighted_tokens']/1_000_000:.1f}MT")
    print(f"  If showing cost:  ◱ ${block_info['cost']:.2f}")
    
    # Debug: Show what Rust code would do
    print_section("What Rust Should Calculate")
    print(f"  1. Load entries from last 24 hours: {len(all_entries)} entries")
    print(f"  2. Identify blocks (5-hour sessions): {len(blocks)} blocks")
    print(f"  3. Find active block: {len(active_block)} entries")
    print(f"  4. Calculate weighted tokens: {block_info['weighted_tokens']:,}")
    print(f"  5. Format as MT: {block_info['weighted_tokens']/1_000_000:.1f}MT")

if __name__ == "__main__":
    main()
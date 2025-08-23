#!/usr/bin/env python3
"""
Detailed token analysis to understand the exact calculations
"""

import json
import os
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Dict, List, Optional, Tuple
from collections import defaultdict

# Color codes
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

def analyze_transcript_file(file_path: Path, cutoff_time: datetime) -> Dict:
    """Analyze a single transcript file"""
    entries = []
    
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            for line_num, line in enumerate(f, 1):
                if not line.strip():
                    continue
                try:
                    entry = json.loads(line)
                    
                    # Parse timestamp
                    timestamp_str = entry.get('timestamp')
                    if timestamp_str:
                        if timestamp_str.endswith('Z'):
                            timestamp = datetime.fromisoformat(timestamp_str.replace('Z', '+00:00'))
                        else:
                            timestamp = datetime.fromisoformat(timestamp_str)
                        
                        if timestamp >= cutoff_time:
                            entry['parsed_timestamp'] = timestamp
                            entry['line_num'] = line_num
                            entries.append(entry)
                            
                except (json.JSONDecodeError, ValueError):
                    continue
                    
    except Exception as e:
        print(f"{RED}Error reading {file_path}: {e}{RESET}")
        return None
    
    if not entries:
        return None
    
    # Sort by timestamp
    entries.sort(key=lambda x: x.get('parsed_timestamp', datetime.min.replace(tzinfo=timezone.utc)))
    
    # Analyze token progression
    token_progression = []
    prev_input = 0
    prev_output = 0
    prev_cache_create = 0
    prev_cache_read = 0
    
    for i, entry in enumerate(entries):
        message = entry.get('message', {})
        usage = message.get('usage', {})
        
        if usage:
            input_tokens = usage.get('input_tokens', 0) or 0
            output_tokens = usage.get('output_tokens', 0) or 0
            cache_create = usage.get('cache_creation_input_tokens', 0) or 0
            cache_read = usage.get('cache_read_input_tokens', 0) or 0
            
            # Calculate deltas
            delta_input = input_tokens - prev_input if input_tokens >= prev_input else input_tokens
            delta_output = output_tokens - prev_output if output_tokens >= prev_output else output_tokens
            delta_cache_create = cache_create - prev_cache_create if cache_create >= prev_cache_create else cache_create
            delta_cache_read = cache_read - prev_cache_read if cache_read >= prev_cache_read else cache_read
            
            token_progression.append({
                'line_num': entry['line_num'],
                'timestamp': entry['parsed_timestamp'],
                'model': message.get('model', 'unknown'),
                'cumulative': {
                    'input': input_tokens,
                    'output': output_tokens,
                    'cache_create': cache_create,
                    'cache_read': cache_read,
                    'total': input_tokens + output_tokens + cache_create + cache_read
                },
                'delta': {
                    'input': delta_input,
                    'output': delta_output,
                    'cache_create': delta_cache_create,
                    'cache_read': delta_cache_read,
                    'total': delta_input + delta_output + delta_cache_create + delta_cache_read
                },
                'is_reset': input_tokens < prev_input or output_tokens < prev_output
            })
            
            prev_input = input_tokens
            prev_output = output_tokens
            prev_cache_create = cache_create
            prev_cache_read = cache_read
    
    return {
        'file': str(file_path),
        'entries': len(entries),
        'token_progression': token_progression,
        'first_timestamp': entries[0]['parsed_timestamp'] if entries else None,
        'last_timestamp': entries[-1]['parsed_timestamp'] if entries else None
    }

def main():
    print_header("Detailed Token Analysis for Claude Powerline")
    
    # Set time window (24 hours)
    now = datetime.now(timezone.utc)
    cutoff_time = now - timedelta(hours=24)
    
    print(f"Analyzing entries from: {cutoff_time.strftime('%Y-%m-%d %H:%M:%S')} UTC")
    print(f"Current time: {now.strftime('%Y-%m-%d %H:%M:%S')} UTC")
    
    # Find all transcript files
    transcript_files = find_all_transcript_files()
    print(f"\nFound {len(transcript_files)} transcript files")
    
    # Analyze each file
    all_analyses = []
    total_cumulative = defaultdict(int)
    total_delta = defaultdict(int)
    
    for file_path in transcript_files:
        analysis = analyze_transcript_file(file_path, cutoff_time)
        if analysis and analysis['token_progression']:
            all_analyses.append(analysis)
            
            # Get the last entry's cumulative values
            last_entry = analysis['token_progression'][-1]
            for key in ['input', 'output', 'cache_create', 'cache_read']:
                total_cumulative[key] += last_entry['cumulative'][key]
            
            # Sum all deltas
            for entry in analysis['token_progression']:
                for key in ['input', 'output', 'cache_create', 'cache_read']:
                    total_delta[key] += entry['delta'][key]
    
    # Print detailed results
    print_header("Session Analysis")
    
    for analysis in all_analyses:
        if not analysis['token_progression']:
            continue
            
        file_name = Path(analysis['file']).name
        project_name = Path(analysis['file']).parent.name
        
        print(f"\n{BOLD}{YELLOW}Project: {project_name}{RESET}")
        print(f"File: {file_name}")
        print(f"Entries: {analysis['entries']}")
        print(f"Time range: {analysis['first_timestamp'].strftime('%H:%M')} - {analysis['last_timestamp'].strftime('%H:%M')}")
        
        # Show first and last few entries
        progression = analysis['token_progression']
        
        if len(progression) > 0:
            print(f"\n  {BOLD}Token Progression:{RESET}")
            
            # First entry
            e = progression[0]
            print(f"  Entry 1 (line {e['line_num']}): {e['timestamp'].strftime('%H:%M:%S')}")
            print(f"    Model: {e['model']}")
            print(f"    Cumulative: input={e['cumulative']['input']:,}, output={e['cumulative']['output']:,}")
            print(f"    Cache: create={e['cumulative']['cache_create']:,}, read={e['cumulative']['cache_read']:,}")
            print(f"    Total: {e['cumulative']['total']:,}")
            
            if len(progression) > 1:
                # Show a middle entry
                mid = len(progression) // 2
                e = progression[mid]
                print(f"\n  Entry {mid+1} (line {e['line_num']}): {e['timestamp'].strftime('%H:%M:%S')}")
                print(f"    Cumulative: {e['cumulative']['total']:,} (Δ+{e['delta']['total']:,})")
                
                # Last entry
                e = progression[-1]
                print(f"\n  Entry {len(progression)} (line {e['line_num']}): {e['timestamp'].strftime('%H:%M:%S')}")
                print(f"    Model: {e['model']}")
                print(f"    Cumulative: input={e['cumulative']['input']:,}, output={e['cumulative']['output']:,}")
                print(f"    Cache: create={e['cumulative']['cache_create']:,}, read={e['cumulative']['cache_read']:,}")
                print(f"    Total: {e['cumulative']['total']:,}")
        
        # Calculate session totals
        session_delta_total = sum(e['delta']['total'] for e in progression)
        session_cumulative_total = progression[-1]['cumulative']['total'] if progression else 0
        
        print(f"\n  {BOLD}Session Summary:{RESET}")
        print(f"    Final cumulative: {session_cumulative_total:,}")
        print(f"    Sum of deltas: {session_delta_total:,}")
        print(f"    Resets detected: {sum(1 for e in progression if e['is_reset'])}")
    
    # Print totals
    print_header("GRAND TOTALS")
    
    print(f"{BOLD}If we sum last cumulative values from each session:{RESET}")
    print(f"  Input tokens: {total_cumulative['input']:,}")
    print(f"  Output tokens: {total_cumulative['output']:,}")
    print(f"  Cache creation: {total_cumulative['cache_create']:,}")
    print(f"  Cache read: {total_cumulative['cache_read']:,}")
    print(f"  TOTAL: {sum(total_cumulative.values()):,}")
    
    print(f"\n{BOLD}If we sum all deltas:{RESET}")
    print(f"  Input tokens: {total_delta['input']:,}")
    print(f"  Output tokens: {total_delta['output']:,}")
    print(f"  Cache creation: {total_delta['cache_create']:,}")
    print(f"  Cache read: {total_delta['cache_read']:,}")
    print(f"  TOTAL: {sum(total_delta.values()):,}")
    
    print(f"\n{BOLD}{RED}Problem Identified:{RESET}")
    print(f"  Summing cumulative values: {sum(total_cumulative.values()):,} tokens")
    print(f"  Summing deltas: {sum(total_delta.values()):,} tokens")
    print(f"  Difference: {sum(total_cumulative.values()) - sum(total_delta.values()):,} tokens")
    
    # Export data for O3 analysis
    export_data = {
        'timestamp': now.isoformat(),
        'cutoff_time': cutoff_time.isoformat(),
        'sessions': all_analyses,
        'total_cumulative': dict(total_cumulative),
        'total_delta': dict(total_delta),
        'files_analyzed': len(all_analyses)
    }
    
    output_file = Path('token_analysis_export.json')
    with open(output_file, 'w') as f:
        json.dump(export_data, f, indent=2, default=str)
    
    print(f"\n{GREEN}Data exported to: {output_file}{RESET}")

if __name__ == "__main__":
    main()
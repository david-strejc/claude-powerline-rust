use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::process::Command;
use tempfile::TempDir;
use tokio::fs;
use std::time::Duration;

fn create_test_transcript(size: usize) -> String {
    let mut transcript_lines = Vec::new();
    let base_time = chrono::Utc::now() - chrono::Duration::days(1);
    
    for i in 0..size {
        let timestamp = base_time + chrono::Duration::minutes(i as i64 * 5);
        let entry = format!(
            r#"{{"timestamp":"{}","message":{{"id":"msg-{}","usage":{{"input_tokens":{},"output_tokens":{},"cache_creation_input_tokens":{},"cache_read_input_tokens":{}}},"model":"claude-3-5-sonnet"}},"costUSD":{},"requestId":"req-{}"}},"#,
            timestamp.format("%Y-%m-%dT%H:%M:%S%.3fZ"),
            i,
            500 + i * 10,
            250 + i * 5, 
            i * 2,
            i * 3,
            0.025 + (i as f64 * 0.001),
            i
        );
        transcript_lines.push(entry);
    }
    
    transcript_lines.join("\n")
}

async fn setup_test_environment(transcript_size: usize) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("projects").join("bench-test");
    fs::create_dir_all(&project_dir).await.unwrap();
    
    let transcript_content = create_test_transcript(transcript_size);
    let transcript_path = project_dir.join("bench-session.jsonl");
    fs::write(&transcript_path, &transcript_content).await.unwrap();
    
    temp_dir
}

fn bench_small_transcript(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let temp_dir = rt.block_on(setup_test_environment(10));
    
    std::env::set_var("CLAUDE_CONFIG_DIR", temp_dir.path().to_str().unwrap());
    
    c.bench_function("rust_small_transcript", |b| {
        b.iter(|| {
            let output = Command::new("./target/release/claude-powerline")
                .output()
                .expect("Failed to execute");
            black_box(output)
        })
    });
    
    std::env::remove_var("CLAUDE_CONFIG_DIR");
}

fn bench_medium_transcript(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let temp_dir = rt.block_on(setup_test_environment(100));
    
    std::env::set_var("CLAUDE_CONFIG_DIR", temp_dir.path().to_str().unwrap());
    
    c.bench_function("rust_medium_transcript", |b| {
        b.iter(|| {
            let output = Command::new("./target/release/claude-powerline")
                .output()
                .expect("Failed to execute");
            black_box(output)
        })
    });
    
    std::env::remove_var("CLAUDE_CONFIG_DIR");
}

fn bench_large_transcript(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let temp_dir = rt.block_on(setup_test_environment(1000));
    
    std::env::set_var("CLAUDE_CONFIG_DIR", temp_dir.path().to_str().unwrap());
    
    // Allow longer measurement time for large transcripts
    let mut group = c.benchmark_group("large_transcript");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(10);
    
    group.bench_function("rust_large_transcript", |b| {
        b.iter(|| {
            let output = Command::new("./target/release/claude-powerline")
                .output()
                .expect("Failed to execute");
            black_box(output)
        })
    });
    
    group.finish();
    std::env::remove_var("CLAUDE_CONFIG_DIR");
}

fn bench_different_themes(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let temp_dir = rt.block_on(setup_test_environment(50));
    
    std::env::set_var("CLAUDE_CONFIG_DIR", temp_dir.path().to_str().unwrap());
    
    let themes = ["dark", "light", "nord", "tokyo-night", "rose-pine"];
    
    let mut group = c.benchmark_group("themes");
    
    for theme in &themes {
        group.bench_with_input(format!("theme_{}", theme), theme, |b, theme| {
            b.iter(|| {
                let output = Command::new("./target/release/claude-powerline")
                    .args(&["--theme", theme])
                    .output()
                    .expect("Failed to execute");
                black_box(output)
            })
        });
    }
    
    group.finish();
    std::env::remove_var("CLAUDE_CONFIG_DIR");
}

fn bench_with_git_operations(c: &mut Criterion) {
    // This benchmark runs in the actual git repository to test git operations
    c.bench_function("rust_with_git", |b| {
        b.iter(|| {
            let output = Command::new("./target/release/claude-powerline")
                .args(&["--theme", "dark"])
                .output()
                .expect("Failed to execute");
            black_box(output)
        })
    });
}

fn bench_concurrent_execution(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let temp_dir = rt.block_on(setup_test_environment(100));
    
    std::env::set_var("CLAUDE_CONFIG_DIR", temp_dir.path().to_str().unwrap());
    
    // Test multiple concurrent executions (simulating multiple terminal sessions)
    c.bench_function("rust_concurrent_5x", |b| {
        b.iter(|| {
            let handles: Vec<_> = (0..5).map(|_| {
                std::thread::spawn(|| {
                    Command::new("./target/release/claude-powerline")
                        .output()
                        .expect("Failed to execute")
                })
            }).collect();
            
            let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
            black_box(results)
        })
    });
    
    std::env::remove_var("CLAUDE_CONFIG_DIR");
}

// Benchmark memory-mapped parsing vs regular file reading
fn bench_parsing_methods(c: &mut Criterion) {
    use claude_powerline_rust::utils::claude::*;
    
    let transcript_content = create_test_transcript(500);
    
    let mut group = c.benchmark_group("parsing_methods");
    
    group.bench_function("jsonl_content_parsing", |b| {
        b.iter(|| {
            let result = parse_jsonl_content(black_box(&transcript_content));
            black_box(result)
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_small_transcript,
    bench_medium_transcript,
    bench_large_transcript,
    bench_different_themes,
    bench_with_git_operations,
    bench_concurrent_execution,
    bench_parsing_methods
);
criterion_main!(benches);
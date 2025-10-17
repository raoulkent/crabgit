#!/bin/bash

# GitCrab Performance Benchmark Script
# 
# This script measures GitCrab performance across different repository sizes
# and analysis types to ensure consistent performance characteristics.

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BENCHMARK_DIR="$(mktemp -d)"
GITCRAB_BIN="${GITCRAB_BIN:-./target/release/gitcrab}"
OUTPUT_FILE="${OUTPUT_FILE:-benchmark_results.json}"

echo -e "${BLUE}🦀 GitCrab Performance Benchmark${NC}"
echo "==============================================="
echo "Benchmark directory: $BENCHMARK_DIR"
echo "GitCrab binary: $GITCRAB_BIN"
echo "Output file: $OUTPUT_FILE"
echo ""

# Ensure GitCrab is built in release mode
if [ ! -f "$GITCRAB_BIN" ]; then
    echo -e "${YELLOW}Building GitCrab in release mode...${NC}"
    cargo build --release
fi

# Create test repositories of different sizes
create_small_repo() {
    local repo_path="$1"
    mkdir -p "$repo_path"
    cd "$repo_path"
    
    git init --quiet
    git config user.name "Benchmark User"
    git config user.email "benchmark@gitcrab.dev"
    
    # Create ~50 commits
    for i in {1..50}; do
        echo "Content $i" > "file_$i.txt"
        git add .
        git commit --quiet -m "Commit $i: Add file_$i.txt"
        
        # Add some churn
        if [ $((i % 5)) -eq 0 ]; then
            echo "Updated content $i" >> "file_$((i-2)).txt"
            git add .
            git commit --quiet -m "Update file_$((i-2)).txt"
        fi
    done
    
    echo "Created small repo with $(git rev-list --count HEAD) commits"
}

create_medium_repo() {
    local repo_path="$1"
    mkdir -p "$repo_path"
    cd "$repo_path"
    
    git init --quiet
    git config user.name "Benchmark User"
    git config user.email "benchmark@gitcrab.dev"
    
    # Create directory structure
    mkdir -p src/{components,utils,tests} docs scripts
    
    # Create ~500 commits with more realistic structure
    for i in {1..500}; do
        case $((i % 4)) in
            0) dir="src/components" ;;
            1) dir="src/utils" ;;
            2) dir="src/tests" ;;
            3) dir="docs" ;;
        esac
        
        echo "// Code for feature $i" > "$dir/module_$i.rs"
        git add .
        git commit --quiet -m "feat: Add module $i to $dir"
        
        # Add refactoring commits
        if [ $((i % 10)) -eq 0 ]; then
            echo "// Refactored code $i" > "$dir/module_$((i-5)).rs"
            git add .
            git commit --quiet -m "refactor: Improve module $((i-5))"
        fi
        
        # Add bug fixes
        if [ $((i % 15)) -eq 0 ]; then
            echo "// Fixed bug in module $i" >> "$dir/module_$((i-3)).rs"
            git add .
            git commit --quiet -m "fix: Resolve issue in module $((i-3))"
        fi
    done
    
    # Add some tags
    git tag -a "v1.0.0" -m "Release 1.0.0" HEAD~400
    git tag -a "v1.1.0" -m "Release 1.1.0" HEAD~300
    git tag -a "v2.0.0" -m "Release 2.0.0" HEAD~100
    git tag -a "v2.1.0" -m "Release 2.1.0"
    
    echo "Created medium repo with $(git rev-list --count HEAD) commits"
}

create_large_repo() {
    local repo_path="$1"
    mkdir -p "$repo_path"
    cd "$repo_path"
    
    git init --quiet
    git config user.name "Benchmark User"
    git config user.email "benchmark@gitcrab.dev"
    
    # Create realistic project structure
    mkdir -p {src,tests,docs,scripts,config}/{backend,frontend,mobile,shared}
    
    # Create ~2000 commits simulating a larger project
    for i in {1..2000}; do
        # Vary the author occasionally
        if [ $((i % 50)) -eq 0 ]; then
            git config user.name "Contributor $((i/50))"
        fi
        
        # Choose directory based on commit number
        case $((i % 8)) in
            0|1) dir="src/backend" ;;
            2|3) dir="src/frontend" ;;
            4) dir="src/mobile" ;;
            5) dir="tests" ;;
            6) dir="docs" ;;
            7) dir="config" ;;
        esac
        
        echo "Content for commit $i" > "$dir/file_$i.txt"
        git add .
        
        # Vary commit message types
        case $((i % 6)) in
            0) msg="feat: Add feature $i" ;;
            1) msg="fix: Fix bug in module $i" ;;
            2) msg="docs: Update documentation for $i" ;;
            3) msg="refactor: Improve code structure $i" ;;
            4) msg="test: Add tests for feature $i" ;;
            5) msg="chore: Update dependencies $i" ;;
        esac
        
        git commit --quiet -m "$msg"
        
        # Simulate larger changes occasionally
        if [ $((i % 25)) -eq 0 ]; then
            for j in {1..5}; do
                echo "Large change $i.$j" >> "$dir/file_$((i-j)).txt"
            done
            git add .
            git commit --quiet -m "refactor: Major refactoring $i"
        fi
        
        # Show progress
        if [ $((i % 200)) -eq 0 ]; then
            echo "  Progress: $i/2000 commits created"
        fi
    done
    
    # Add multiple tags
    for tag_num in {1..10}; do
        commit_offset=$((tag_num * 200))
        git tag -a "v$tag_num.0.0" -m "Release $tag_num.0.0" HEAD~$commit_offset
    done
    
    echo "Created large repo with $(git rev-list --count HEAD) commits"
}

# Benchmark function
benchmark_command() {
    local repo_size="$1"
    local repo_path="$2"
    local command="$3"
    local description="$4"
    
    echo -e "${YELLOW}Testing: $description${NC}"
    
    # Warm up (ignore this run)
    cd "$repo_path"
    timeout 60 $GITCRAB_BIN $command >/dev/null 2>&1 || true
    
    # Actual benchmark run
    local start_time=$(date +%s.%N)
    cd "$repo_path"
    
    if timeout 60 $GITCRAB_BIN $command >/dev/null 2>&1; then
        local end_time=$(date +%s.%N)
        local duration=$(echo "$end_time - $start_time" | bc -l)
        
        printf "  ✅ %-50s %6.2fs\n" "$description" "$duration"
        
        # Store result
        echo "{\"repo_size\":\"$repo_size\",\"command\":\"$command\",\"description\":\"$description\",\"duration\":$duration,\"status\":\"success\"}" >> "$BENCHMARK_DIR/results.jsonl"
    else
        echo -e "  ❌ ${RED}$description - TIMEOUT/FAILED${NC}"
        echo "{\"repo_size\":\"$repo_size\",\"command\":\"$command\",\"description\":\"$description\",\"duration\":60,\"status\":\"timeout\"}" >> "$BENCHMARK_DIR/results.jsonl"
    fi
}

# Run benchmarks for a repository
benchmark_repo() {
    local repo_size="$1"
    local repo_path="$2"
    
    echo ""
    echo -e "${GREEN}Benchmarking $repo_size repository${NC}"
    echo "Repository path: $repo_path"
    
    # Basic commands
    benchmark_command "$repo_size" "$repo_path" "" "Repository status"
    benchmark_command "$repo_size" "$repo_path" "branches" "List branches"
    benchmark_command "$repo_size" "$repo_path" "log --count 10" "Recent commits"
    
    # Stats commands - basic
    benchmark_command "$repo_size" "$repo_path" "stats" "Default statistics"
    benchmark_command "$repo_size" "$repo_path" "stats repo --since 90d --format json" "Activity analysis (90d)"
    benchmark_command "$repo_size" "$repo_path" "stats authors --top 10 --format json" "Author analysis"
    benchmark_command "$repo_size" "$repo_path" "stats calendar --since 365d --format json" "Calendar heatmap"
    
    # Stats commands - advanced
    benchmark_command "$repo_size" "$repo_path" "stats hotspots --since 180d --top 25" "File hotspots"
    benchmark_command "$repo_size" "$repo_path" "stats branches --format json" "Branch analysis"
    benchmark_command "$repo_size" "$repo_path" "stats coupling --top 10" "Coupling analysis"
    benchmark_command "$repo_size" "$repo_path" "stats ownership --top 15" "Ownership (fast)"
    benchmark_command "$repo_size" "$repo_path" "stats stability --top 20" "Stability analysis"
    benchmark_command "$repo_size" "$repo_path" "stats releases --format json" "Release analysis"
    
    # Performance-focused tests
    benchmark_command "$repo_size" "$repo_path" "--no-cache stats repo --since 30d" "No cache mode"
    benchmark_command "$repo_size" "$repo_path" "--max-threads 1 stats authors" "Single threaded"
    benchmark_command "$repo_size" "$repo_path" "--max-threads 4 stats authors" "Multi threaded"
}

# Main execution
main() {
    echo -e "${BLUE}Setting up test repositories...${NC}"
    
    # Create repositories
    echo "Creating small repository..."
    create_small_repo "$BENCHMARK_DIR/small_repo"
    
    echo "Creating medium repository..."
    create_medium_repo "$BENCHMARK_DIR/medium_repo" 
    
    echo "Creating large repository..."
    create_large_repo "$BENCHMARK_DIR/large_repo"
    
    # Initialize results file
    echo "" > "$BENCHMARK_DIR/results.jsonl"
    
    # Run benchmarks
    benchmark_repo "small" "$BENCHMARK_DIR/small_repo"
    benchmark_repo "medium" "$BENCHMARK_DIR/medium_repo"
    benchmark_repo "large" "$BENCHMARK_DIR/large_repo"
    
    # Generate summary report
    echo ""
    echo -e "${BLUE}Generating benchmark report...${NC}"
    
    cat > "$OUTPUT_FILE" << EOF
{
  "benchmark_info": {
    "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "gitcrab_version": "$($GITCRAB_BIN --version 2>/dev/null || echo 'unknown')",
    "system_info": {
      "os": "$(uname -s)",
      "arch": "$(uname -m)",
      "cores": "$(nproc 2>/dev/null || echo 'unknown')"
    }
  },
  "results": [
EOF
    
    # Convert JSONL to JSON array
    sed 's/$/,/' "$BENCHMARK_DIR/results.jsonl" | sed '$s/,$//' >> "$OUTPUT_FILE"
    
    cat >> "$OUTPUT_FILE" << EOF
  ]
}
EOF

    echo -e "${GREEN}✅ Benchmark complete!${NC}"
    echo "Results saved to: $OUTPUT_FILE"
    
    # Print summary
    echo ""
    echo "Summary:"
    echo "--------"
    
    # Count successes and failures
    local total_tests=$(wc -l < "$BENCHMARK_DIR/results.jsonl")
    local successful_tests=$(grep '"status":"success"' "$BENCHMARK_DIR/results.jsonl" | wc -l)
    local failed_tests=$((total_tests - successful_tests))
    
    echo "Total tests: $total_tests"
    echo -e "Successful: ${GREEN}$successful_tests${NC}"
    if [ $failed_tests -gt 0 ]; then
        echo -e "Failed/Timeout: ${RED}$failed_tests${NC}"
    fi
    
    # Show slowest operations
    echo ""
    echo "Top 5 slowest operations:"
    grep '"status":"success"' "$BENCHMARK_DIR/results.jsonl" | \
        jq -r '"\(.duration | floor) \(.description) (\(.repo_size))"' | \
        sort -nr | head -5 | \
        while read duration desc; do
            printf "  %3ds %s\n" "$duration" "$desc"
        done
}

# Cleanup function
cleanup() {
    echo ""
    echo -e "${YELLOW}Cleaning up...${NC}"
    rm -rf "$BENCHMARK_DIR"
    echo "Done!"
}

# Set trap for cleanup
trap cleanup EXIT

# Check dependencies
if ! command -v git &> /dev/null; then
    echo -e "${RED}Error: git is required but not installed${NC}"
    exit 1
fi

if ! command -v bc &> /dev/null; then
    echo -e "${RED}Error: bc is required but not installed${NC}"
    exit 1
fi

if ! command -v jq &> /dev/null; then
    echo -e "${RED}Error: jq is required but not installed${NC}"
    exit 1
fi

# Run main function
main "$@"
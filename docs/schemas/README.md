# GitCrab JSON Schemas

This directory contains JSON Schema definitions for all GitCrab output formats. These schemas document the structure and validate the JSON output from various GitCrab analysis commands.

## Overview

GitCrab provides structured JSON output for programmatic consumption and integration with other tools. All JSON outputs follow consistent patterns and include:

- **Context information**: Repository path, time windows, analysis parameters
- **Metadata**: Analysis type, configuration options, result counts
- **Data**: The actual analysis results in structured format

## Available Schemas

### Repository Analysis

| Schema | Description | Command Example |
|--------|-------------|-----------------|
| [`repo-activity.json`](./repo-activity.json) | Commit activity over time | `crabgit stats repo --format json` |
| [`authors.json`](./authors.json) | Author contribution statistics | `crabgit stats authors --format json` |
| [`calendar.json`](./calendar.json) | Weekday/hour activity heatmap | `crabgit stats calendar --format json` |

### Planned Schemas (Future Releases)

- `hotspots.json` - File hotspots with churn decay
- `branches.json` - Branch analysis and metrics
- `coupling.json` - File co-change analysis
- `ownership.json` - Code ownership patterns
- `stability.json` - Change stability analysis
- `releases.json` - Release and tag metrics

## Usage Examples

### Basic Repository Activity

```bash
# Get last 90 days of weekly commit activity
crabgit stats repo --since 90d --bucket week --format json > activity.json
```

Example output structure:
```json
{
  "context": {
    "repo_path": "/home/user/project",
    "since": "90d",
    "until": null,
    "bucket": "Week",
    "no_merges": false
  },
  "metric": "activity",
  "bucket": "Week",
  "series": [
    {"bucket_start": 1704067200, "commits": 12},
    {"bucket_start": 1704672000, "commits": 8}
  ]
}
```

### Author Analysis

```bash
# Get top 10 authors by commit count
crabgit stats authors --top 10 --metric commits --format json > authors.json
```

### Calendar Heatmap

```bash
# Activity pattern by weekday and hour
crabgit stats calendar --since 365d --format json > heatmap.json
```

## Schema Validation

To validate GitCrab JSON output against these schemas, you can use tools like:

- [ajv-cli](https://www.npmjs.com/package/ajv-cli): `ajv validate -s schema.json -d data.json`
- [jsonschema](https://pypi.org/project/jsonschema/) (Python): Programmatic validation
- Online validators: [JSON Schema Validator](https://jsonschemalint.com/)

## Integration Examples

### Python Analysis

```python
import json
import subprocess

# Run GitCrab analysis
result = subprocess.run([
    'crabgit', 'stats', 'authors', 
    '--format', 'json', '--top', '20'
], capture_output=True, text=True)

data = json.loads(result.stdout)
authors = data['authors']

# Extract top contributor
top_author = authors[0]
print(f"Top contributor: {top_author['author']} ({top_author['commits']} commits)")
```

### JavaScript/Node.js

```javascript
const { execSync } = require('child_process');

// Get repository activity data
const output = execSync('crabgit stats repo --since 90d --format json', { encoding: 'utf8' });
const data = JSON.parse(output);

// Process time series
const totalCommits = data.series.reduce((sum, point) => sum + point.commits, 0);
console.log(`Total commits in last 90 days: ${totalCommits}`);
```

### jq Queries

```bash
# Extract commit counts from activity data
crabgit stats repo --format json | jq '.series[].commits'

# Get author names and commit counts
crabgit stats authors --format json | jq '.authors[] | {name: .author, commits: .commits}'

# Find peak activity hour from calendar
crabgit stats calendar --format json | jq '.matrix | flatten | max'
```

## Schema Development

When adding new analysis types to GitCrab:

1. Define the output structure in Rust using `serde::Serialize`
2. Create corresponding JSON Schema in this directory
3. Include realistic example data in the schema
4. Update this README with usage examples
5. Add integration tests verifying schema compliance

## Common Patterns

All GitCrab JSON outputs follow these conventions:

- **Timestamps**: Unix timestamps (seconds since epoch) for consistency
- **Counts**: Non-negative integers for all count fields
- **Context**: Every output includes analysis context for reproducibility
- **Null handling**: Optional fields use `null` rather than omission
- **Sorting**: Results are pre-sorted by the primary metric (descending)

## Versioning

These schemas follow semantic versioning. Breaking changes to output format will increment the major version and update schema URLs accordingly.
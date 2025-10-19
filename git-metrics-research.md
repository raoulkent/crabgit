# Git Repository Metrics Research

## Executive Summary

This document presents comprehensive research into Git repository analysis tools, established software metrics, and repository health indicators. The goal is to inform the development of a data-driven scoring system for analyzing codebases, contributions, and contributors to enable better decision-making around code quality and sustainability.

## Table of Contents

1. [Methods](#methods)
2. [Current CrabGit Implementation](#current-crabgit-implementation)
3. [Existing Git Analysis Tools](#existing-git-analysis-tools)
4. [Established Software Metrics](#established-software-metrics)
5. [Platform Metrics (GitHub, GitLab)](#platform-metrics)
6. [Academic Research](#academic-research)
7. [Comprehensive Metrics Map](#comprehensive-metrics-map)
8. [Scoring System Framework](#scoring-system-framework)
9. [Feasibility & Data Sources](#feasibility--data-sources)
10. [Performance, Scale, and Data Quality](#performance-scale-and-data-quality)
11. [Ethics and Responsible Use](#ethics-and-responsible-use)
12. [Validation and Calibration Plan](#validation-and-calibration-plan)
13. [Implementation Recommendations](#implementation-recommendations)
14. [References](#references)

---

## Methods

This research used a combination of GitHub API searches and literature review. Selection favored actively maintained, broadly used, and conceptually influential tools and frameworks.

Search approach (examples):
- GitHub API queries: 
  - "git statistics analysis" (languages: Python/Rust/Go), sorted by stars
  - Specific tools by name (e.g., code-maat, hercules, git-of-theseus, gitinspector)
  - Ecosystem frameworks (CHAOSS GrimoireLab, Augur)
- Inclusion criteria: maintained within last 24 months, significant adoption (stars, community), relevance to Git history analysis or code quality metrics.
- Exclusion criteria: toy/student projects, inactive forks, platform-only analytics without Git history relevance.

All star counts and repository metadata are as of 2025-10-19.

---

## Current CrabGit Implementation

### Existing Metrics
Based on analysis of the current codebase, CrabGit already implements several sophisticated metrics:

#### Repository-Level Metrics
- **Total commits** - Basic activity measurement
- **Branch analysis** - Local vs remote branch counts
- **Repository age** - Time since first commit
- **Activity analysis** - Commit activity over time with configurable buckets (day/week/month)
- **Churn analysis** - Code change frequency

#### Author/Contributor Metrics  
- **Top contributors** - Ranked by commit count and churn
- **Author statistics** - Commits and code churn per author
- **Code ownership** - File ownership analysis (fast and expensive modes)

#### File-Level Metrics
- **File hotspots** - High churn files with recency decay (half-life based)
- **File coupling** - Co-change analysis between files
- **Change stability** - File and author change pattern analysis

#### Time-Based Analysis
- **Calendar analysis** - Activity heatmaps by weekday/hour
- **Release analysis** - Tag and release metrics
- **Time window filtering** - Configurable since/until date ranges

#### Branch Analysis
- **Branch metrics** - Ahead/behind status, activity, merge ratios
- **Base branch comparison** - Compare branches against main/master

### Architecture Strengths
- Uses Git2 Rust library for direct Git operations (not shell commands)
- Supports parallel processing and caching
- Multiple output formats (JSON, Table, Chart)
- Interactive TUI and CLI interfaces
- Configurable analysis windows and filtering

---

## Existing Git Analysis Tools

As of 2025-10-19. Stars are included to indicate adoption, not quality.

### Comparison Matrix

| Tool | Repo | Type | Primary focus | Representative metrics | Stars (as of 2025-10-19) |
|---|---|---|---|---|---|
| GitInspector | ejwa/gitinspector | CLI (Python) | Repo stats | contributions, authors, file stats, blame | 2484 |
| git-of-theseus | erikbern/git-of-theseus | CLI (Python) | Evolution/survival | line survival, lifetime, author persistence | 2816 |
| Hercules | src-d/hercules | CLI/Lib (Go) | Advanced analysis | ownership, coupling, developer metrics | 2735 |
| Code Maat | adamtornhill/code-maat | CLI (JVM) | Forensic analysis | temporal coupling, hotspots, churn | 2510 |
| GitStats | hoxu/gitstats | CLI (Python) | Basic repo stats | commits, authors, files, activity | 1668 |
| git-fame | casperdcl/git-fame | CLI (Python) | Author stats | LOC per author, files, commits | 767 |
| tokei | XAMPPRocky/tokei | CLI (Rust) | SLOC counting | per-language LOC, files | 13318 |
| scc | boyter/scc | CLI (Go) | SLOC counting | per-language LOC, files | 7729 |
| Gource | acaudwell/Gource | Visualizer | Animation | activity visualization | 12391 |
| GitLens | GitKraken/vscode-gitlens | IDE extension | Code-aware Git | blame/insights in-editor | 9560 |
| GrimoireLab | chaoss/grimoirelab | Platform | Community metrics | Git/GitHub/Issues analytics | 558 |
| Augur | chaoss/augur | Platform | Community metrics | project health dashboards | 652 |

Notable commercial platforms (for positioning): SonarQube, CodeClimate, GitKraken, LinearB/SEI—useful for inspiration but out of scope for OSS Git-only analysis.

### Key Insights
- Most OSS tools report raw counts/aggregations; few provide normalized or comparable scores.
- Evolutionary analysis (Theseus) and temporal coupling (Code Maat) provide high-leverage insights.
- Platform-level suites (CHAOSS) require external APIs; differentiate "Git-only" from "integrations".
- Combining churn, coupling, and ownership yields actionable hotspots with context.

---

## Established Software Metrics

### Classical Software Engineering Metrics

#### Complexity Metrics
1. **Cyclomatic Complexity** - Measure of code complexity based on control flow
2. **Halstead Metrics** - Volume, difficulty, effort measurements
3. **Lines of Code (LOC)** - Physical and logical lines
4. **Function/Class Count** - Structural complexity indicators

#### Quality Metrics  
1. **Technical Debt Ratio** - Effort to fix vs. development effort
2. **Code Coverage** - Test coverage percentage
3. **Duplication Ratio** - Percentage of duplicated code
4. **Maintainability Index** - Composite score of maintainability

#### Change Metrics
1. **Churn Rate** - Code change frequency
2. **Defect Density** - Bugs per lines of code
3. **Change Failure Rate** - Failed deployments/changes
4. **Lead Time** - Time from commit to production

### Git-Specific Metrics

#### Commit Analysis
- **Commit frequency patterns**
- **Commit message quality** (conventional commits, length, descriptiveness)
- **Commit size distribution**
- **Merge vs. feature commit ratios**

#### Branching Patterns
- **Branch lifecycle duration**
- **Branch complexity** (merge patterns)
- **Feature branch size**
- **Integration frequency**

#### Collaboration Patterns
- **Bus factor** - Knowledge concentration risk
- **Pair programming indicators**
- **Code review participation**
- **Cross-team collaboration**

---

## Platform Metrics

These require platform APIs and are not available from Git history alone. Treat them as optional integrations.

### GitHub Metrics (via REST/GraphQL)
- **Stars/Forks** (engagement)
- **Issues/PRs** (throughput, responsiveness)
- **Release frequency** (cadence)
- **Community health files** (governance)
- **Security alerts/Dependabot** (risk)
- **Traffic data** (reach)

### GitLab Metrics
- **Merge request analytics**
- **CI/CD pipeline success rates**
- **Lead time/deployment frequency (DORA)**
- **Issue resolution time**
- **Code review efficiency**

### Notes
- Mark clearly in UI/CLI which metrics need external credentials.
- Keep CrabGit functional in "offline" mode (Git-only) with graceful degradation.

---

## Academic Research

### Key Research Areas

#### 1. Developer Productivity Measurement
- **Research Focus**: Quantifying developer effectiveness
- **Metrics**: 
  - Code throughput (features/time)
  - Code quality (bugs/feature)
  - Learning curve analysis
  - Collaboration effectiveness

#### 2. Technical Debt Research
- **Research Focus**: Quantifying and managing technical debt
- **Metrics**:
  - Architectural debt
  - Code debt (complexity, duplication)
  - Test debt (coverage gaps)
  - Documentation debt

#### 3. Software Evolution Studies
- **Research Focus**: How software systems evolve over time
- **Metrics**:
  - Evolutionary coupling
  - Change prediction models
  - Hotspot identification
  - Refactoring impact analysis

#### 4. Team Dynamics in Software Development
- **Research Focus**: Team collaboration patterns
- **Metrics**:
  - Communication patterns
  - Knowledge sharing indicators
  - Team bus factor
  - Onboarding effectiveness

---

## Comprehensive Metrics Map

### Tier 1: Foundation Metrics (Easy Implementation)
These metrics require only Git history analysis and are fundamental to any scoring system.

#### Repository Health
- ✅ **Total commits** - Basic activity indicator
- ✅ **Repository age** - Maturity indicator
- ✅ **Active contributor count** - Team size
- ✅ **Commit frequency** - Development velocity
- ✅ **Branch count** - Development complexity
- 🔄 **Average commit size** - Development patterns
- 🔄 **Merge frequency** - Integration patterns

#### Contributor Analysis  
- ✅ **Top contributors by commits** - Productivity ranking
- ✅ **Code ownership distribution** - Knowledge concentration
- ✅ **Author churn metrics** - Individual productivity
- 🔄 **Bus factor calculation** - Risk assessment
- 🔄 **Contributor tenure** - Team stability
- 🔄 **New contributor rate** - Community growth

#### File/Code Analysis
- ✅ **File hotspots** - Change concentration
- ✅ **File coupling** - Architectural insights
- ✅ **Change stability** - Code maturity
- 🔄 **File lifetime analysis** - Code evolution
- 🔄 **Directory churn patterns** - Architectural focus
- 🔄 **Binary vs. text file ratios** - Project composition

### Tier 2: Advanced Metrics (Moderate Implementation)
These require more sophisticated analysis but provide deeper insights.

#### Code Evolution
- 🔄 **Code survival rates** (git-of-theseus style) - Code persistence
- 🔄 **Line lifetime distribution** - Code stability
- 🔄 **Feature branch lifecycle** - Development workflow
- 🔄 **Refactoring detection** - Code improvement patterns
- 🔄 **Dead code accumulation** - Technical debt indicator

#### Collaboration Patterns
- 🔄 **Co-authorship networks** - Team collaboration
- 🔄 **Review participation rates** - Code quality culture
- 🔄 **Cross-team file modifications** - Inter-team dependencies
- 🔄 **Mentorship patterns** - Knowledge transfer
- 🔄 **Conflict resolution time** - Team efficiency

#### Temporal Analysis
- ✅ **Activity heatmaps** - Work patterns
- 🔄 **Release cycle analysis** - Delivery predictability  
- 🔄 **Bug fix vs. feature ratios** - Development focus
- 🔄 **Seasonal development patterns** - Team dynamics
- 🔄 **Sprint/milestone completion rates** - Planning accuracy

### Tier 3: Expert Metrics (Complex Implementation)
These require integration with external tools or advanced analysis techniques.

#### Code Quality Integration
- 🔄 **Technical debt evolution** - Quality trends over time
- 🔄 **Code complexity trends** - Maintainability direction
- 🔄 **Test coverage correlation** - Quality assurance patterns
- 🔄 **Security vulnerability introduction** - Risk assessment
- 🔄 **Performance regression tracking** - Quality maintenance

#### Predictive Analytics
- 🔄 **Bug prediction models** - Risk forecasting
- 🔄 **Contributor churn prediction** - Team stability risks
- 🔄 **Code hotspot prediction** - Future maintenance needs
- 🔄 **Release risk assessment** - Deployment confidence
- 🔄 **Technical debt accumulation trends** - Long-term planning

#### Business Metrics Integration
- 🔄 **Feature delivery velocity** - Business value delivery
- 🔄 **Customer issue correlation** - Quality impact
- 🔄 **Performance impact tracking** - User experience metrics
- 🔄 **Security incident correlation** - Risk realization
- 🔄 **Compliance tracking** - Regulatory requirements

Legend: ✅ Implemented, 🔄 Not implemented, ❌ Not applicable

---

## Scoring System Framework

### Multi-Dimensional Scoring Approach

#### 1. Repository Health Score (0-100)
**Components:**
- **Activity Score (25%)**: Based on commit frequency, contributor activity
- **Stability Score (25%)**: Based on change patterns, hotspot concentration  
- **Collaboration Score (25%)**: Based on contributor diversity, bus factor
- **Evolution Score (25%)**: Based on code survival, refactoring patterns

#### 2. Contributor Score (0-100)  
**Components:**
- **Productivity Score (30%)**: Commits, churn, features delivered
- **Quality Score (30%)**: Bug introduction rate, code review participation
- **Collaboration Score (25%)**: Cross-team work, mentorship, knowledge sharing
- **Growth Score (15%)**: Learning curve, skill development over time

#### 3. Code Quality Score (0-100)
**Components:**
- **Maintainability (40%)**: Complexity trends, refactoring frequency
- **Reliability (30%)**: Bug rates, test coverage correlation
- **Security (20%)**: Vulnerability introduction, security practice adherence  
- **Performance (10%)**: Performance impact of changes

### Weighting Strategies
- **Recency weighting**: Recent changes matter more (exponential decay)
- **Size weighting**: Larger changes have more impact
- **Context weighting**: Critical files/modules weighted higher
- **Risk weighting**: Security/performance changes weighted higher

### Scoring details and formulas
- Normalization (robust): m' = clip_0_1((m - p10) / (p90 - p10)) computed over a 12-month rolling window.
- Recency decay: w(t_days) = 0.5^(t_days / H), default H = 90 days.
- Hotspot score(file): sum_i churn_i(file) + sum_i deletions_i(file), each term weighted by w(age_i); normalize per-file to [0,1].
- Coupling: support(A,B) = co_changes(A,B)/N; confidence(A→B) = co_changes(A,B)/changes(A); lift = confidence / (changes(B)/N).
- Ownership (fast): primary_owner(file) = last modifier; (expensive) blame percentage per author with tie-break by most recent.
- Stability(file): median inter-change interval (days) or stability = 1 / (changes_per_day_per_KLOC).
- Bus factor: minimal N authors covering ≥ X% (default 70%) of LOC/files; report distributions.
- Repository Health Score: weighted sum of normalized sub-metrics; defaults Activity 25, Stability 25, Collaboration 25, Evolution 25.

Guardrails
- Exclude bots (author name contains "[bot]" or known list: dependabot, renovate, greenkeeper; configurable).
- Exclude generated/binary content (glob defaults: node_modules/**, dist/**, target/**, vendor/**, *.min.*, *.lock, *.png, *.jpg, *.pdf, *.zip).
- Treat renames/moves using similarity index (e.g., ≥70%) to carry history across paths.

## Feasibility & Data Sources

- Git-only (supported offline): commits, churn, hotspots with decay, temporal coupling, ownership (fast or blame), stability, calendar heatmaps, branches, releases/tags.
- Requires integrations: PR review metrics, CI/CD/DORA, issue linkage/defect density, coverage, security scans. Clearly mark as optional.

## Performance, Scale, and Data Quality

- Rename/move handling: detect with similarity and follow across history; collapse massive renames into mapping tables.
- Large repos: streaming revwalks, windowed coupling (configurable window_size), caching of commit metadata; target budgets: Linux kernel ≤10 min cold start, ≤2 min warm.
- Incremental updates: process only new commits since last analysis; persist intermediate indices (SQLite recommended).
- Correctness checks: sample cross-compare with Code Maat and GitInspector on selected windows.

## Ethics and Responsible Use

- Default to team-level aggregates; avoid individual leaderboards.
- Provide anonymization and opt-out lists; document appropriate use and anti-gaming.
- Explain metrics and uncertainty; avoid using for performance reviews.

## Validation and Calibration Plan

- Calibration datasets: linux, kubernetes, react, numpy, rust (varied sizes and cultures).
- Baselines: reproduce known patterns (e.g., hotspot modules) using Code Maat/Theseus for sanity checks.
- Sensitivity analysis: vary half-life, window sizes, and outlier trimming; publish parameter defaults.
- Bug-introducing commits (optional): prototype SZZ variant; document limitations and require issue linkage for accuracy.

---

## Implementation Recommendations

### Phase 1: Enhance Foundation (Next 2-4 weeks)
1. **Implement missing Tier 1 metrics**
   - Average commit size analysis
   - Bus factor calculation  
   - Contributor tenure tracking
   - File lifetime analysis

2. **Add scoring framework**
   - Implement multi-dimensional scoring
   - Add configurable weighting
   - Create score visualization

3. **Enhance existing metrics**
   - Add recency weighting to hotspots
   - Improve coupling analysis with confidence scoring
   - Add trend analysis to existing metrics

### Phase 2: Advanced Analytics (1-2 months)
1. **Implement Tier 2 metrics**
   - Code survival analysis (git-of-theseus style)
   - Collaboration network analysis
   - Release cycle analysis
   - Refactoring detection

2. **Add predictive capabilities**
   - Hotspot prediction
   - Contributor churn risk
   - Bug prediction models

3. **Enhanced reporting**
   - Multi-repository analysis
   - Executive dashboards
   - Trend alerts and notifications

### Phase 3: Expert Integration (2-3 months)
1. **External tool integration**
   - SonarQube metrics integration
   - CI/CD pipeline data
   - Issue tracker correlation
   - Code review platform integration

2. **Machine learning features**
   - Anomaly detection
   - Pattern recognition
   - Recommendation engine
   - Automated insights

3. **Enterprise features**
   - Team benchmarking
   - Portfolio analysis
   - Custom metric definitions
   - Advanced visualization

### Technical Implementation Strategy

#### Data Collection Pipeline
1. **Git data extraction** - Leverage existing git2 integration
2. **Caching strategy** - Enhance existing cache with metric-specific caching
3. **Incremental updates** - Only analyze new commits
4. **Parallel processing** - Leverage existing parallel framework

#### Storage and Performance
1. **Metric storage** - SQLite for local storage, optional external DB
2. **Index optimization** - Pre-compute common metric combinations
3. **Memory management** - Stream processing for large repositories
4. **Query optimization** - Efficient metric retrieval and aggregation

#### API and Integration
1. **REST API** - Expose metrics programmatically
2. **Webhook support** - Real-time updates
3. **Export formats** - JSON, CSV, XML support
4. **Plugin architecture** - Custom metric extensions

---

## Conclusion

The research reveals a significant opportunity to differentiate CrabGit through:

1. **Comprehensive scoring system** - Most tools focus on raw metrics, few provide scoring
2. **Advanced evolution tracking** - Git-of-theseus style analysis is rare but valuable
3. **Predictive analytics** - Few tools provide forward-looking insights
4. **Multi-dimensional analysis** - Holistic view of repository health

CrabGit's existing foundation is strong, with sophisticated metrics already implemented. The proposed phased approach will systematically build upon this foundation to create a best-in-class repository analysis and scoring platform.

The focus should be on actionable insights rather than just data collection - helping teams make better decisions about refactoring, code reviews, team structure, and technical debt management.

---

Compiled: 2025-10-19
CrabGit version analyzed: current main branch

## References
- GitInspector — https://github.com/ejwa/gitinspector (accessed 2025-10-19)
- git-of-theseus — https://github.com/erikbern/git-of-theseus (accessed 2025-10-19)
- Hercules — https://github.com/src-d/hercules (accessed 2025-10-19)
- Code Maat — https://github.com/adamtornhill/code-maat (accessed 2025-10-19)
- GitStats — https://github.com/hoxu/gitstats (accessed 2025-10-19)
- git-fame — https://github.com/casperdcl/git-fame (accessed 2025-10-19)
- tokei — https://github.com/XAMPPRocky/tokei (accessed 2025-10-19)
- scc — https://github.com/boyter/scc (accessed 2025-10-19)
- Gource — https://github.com/acaudwell/Gource (accessed 2025-10-19)
- GitLens — https://github.com/GitKraken/vscode-gitlens (accessed 2025-10-19)
- CHAOSS GrimoireLab — https://github.com/chaoss/grimoirelab (accessed 2025-10-19)
- CHAOSS Augur — https://github.com/chaoss/augur (accessed 2025-10-19)
- CHAOSS Metrics — https://github.com/chaoss/metrics (accessed 2025-10-19)
- Conventional Commits — https://www.conventionalcommits.org/ (accessed 2025-10-19)
- Accelerate (DORA) — https://itrevolution.com/accelerate/ (accessed 2025-10-19)
- The SPACE of Developer Productivity — https://queue.acm.org/detail.cfm?id=3454124 (accessed 2025-10-19)
- Sliwerski, Zimmermann, Zeller (2005), "When do changes induce fixes?" — https://doi.org/10.1145/1082983.1083147 (accessed 2025-10-19)
- Tornhill, "Your Code as a Crime Scene" — https://pragprog.com/titles/atcrime/your-code-as-a-crime-scene/ (accessed 2025-10-19)

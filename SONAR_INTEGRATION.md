# SonarCloud Integration Guide

This guide explains how to use the improved `sonar.py` script to fetch and analyze SonarCloud issues with GitHub Copilot.

## Overview

The `sonar.py` script has been redesigned to:

- ✅ **Fetch fresh analysis results** with staleness detection
- ✅ **Provide rich metadata** optimized for GitHub Copilot
- ✅ **Support filtering** by severity, type, and status
- ✅ **Include comprehensive rule descriptions** and remediation guidance
- ✅ **Detect stale results** and warn when analysis is outdated
- ✅ **Support both branches and pull requests**

## Quick Start

### 1. Set up environment variables

```bash
export SONAR_TOKEN=<your-sonarcloud-token>
export SONAR_ORG=edgefirstai
export SONAR_PROJECT=EdgeFirstAI_client
```

To get a SonarCloud token:
1. Visit [SonarCloud](https://sonarcloud.io/)
2. Go to: **Account → Security → Generate Tokens**
3. Create a new token with read permissions

### 2. Fetch issues for the current branch

```bash
# Fetch all open issues
python3 sonar.py --branch main --output sonar-issues.json --verbose

# View the summary in the terminal
python3 sonar.py --branch main --output sonar-issues.json -v
```

### 3. Use with GitHub Copilot

Once you have the `sonar-issues.json` file:

1. Open it in VS Code or your IDE
2. Ask Copilot: "@workspace Review the sonar-issues.json file and help me fix the top 5 critical issues"
3. Copilot will read the structured data and provide specific fixes

## Usage Examples

### Filter by severity

```bash
# Only show critical and high-severity issues
python3 sonar.py --branch main \
  --severity BLOCKER,CRITICAL \
  --output critical-issues.json
```

### Filter by issue type

```bash
# Only show bugs and vulnerabilities
python3 sonar.py --branch main \
  --type BUG,VULNERABILITY \
  --output bugs-and-vulns.json
```

### Analyze a pull request

```bash
# Fetch issues for a specific PR
python3 sonar.py --pull-request 123 \
  --output pr-123-issues.json
```

### Include resolved issues

```bash
# Include both open and resolved issues (for historical analysis)
python3 sonar.py --branch main \
  --include-resolved \
  --output all-issues.json
```

### Check for stale analysis

```bash
# Warn if analysis is older than 12 hours
python3 sonar.py --branch main \
  --max-age-hours 12 \
  --output sonar-issues.json -v
```

## Output Format

The script generates JSON in a Copilot-optimized format:

```json
{
  "version": "1.0",
  "generatedAt": "2025-10-25T18:30:00Z",
  "project": {
    "key": "EdgeFirstAI_client",
    "organization": "edgefirstai",
    "branch": "main"
  },
  "summary": {
    "totalIssues": 42,
    "totalHotspots": 3,
    "bySeverity": {
      "CRITICAL": 2,
      "MAJOR": 15,
      "MINOR": 25
    },
    "byType": {
      "BUG": 5,
      "CODE_SMELL": 35,
      "VULNERABILITY": 2
    },
    "qualityGateStatus": "OK",
    "analysisDate": "2025-10-25T12:00:00Z",
    "isStale": false,
    "ageHours": 6.5
  },
  "issues": [
    {
      "file": "src/main.rs",
      "line": 142,
      "endLine": 142,
      "column": 12,
      "endColumn": 25,
      "severity": "MAJOR",
      "type": "CODE_SMELL",
      "rule": "rust:S1541",
      "ruleName": "Functions should not be too complex",
      "message": "This function has a cyclomatic complexity of 15.",
      "status": "OPEN",
      "effort": "30min",
      "tags": ["brain-overload"],
      "context": {
        "ruleDescription": "<p>Cognitive Complexity is a measure...</p>",
        "language": "rust"
      }
    }
  ],
  "hotspots": []
}
```

### Key fields for Copilot:

- **file**: Relative path from project root
- **line** / **endLine**: Precise line numbers (1-based)
- **column** / **endColumn**: Character positions
- **severity**: BLOCKER, CRITICAL, MAJOR, MINOR, INFO
- **type**: BUG, VULNERABILITY, CODE_SMELL, SECURITY_HOTSPOT
- **message**: Human-readable description
- **ruleName**: Short rule name
- **context.ruleDescription**: Full HTML description with remediation guidance

## Integration with GitHub Copilot

### VS Code with Copilot Chat

1. **Generate the report:**
   ```bash
   python3 sonar.py --branch main -o sonar-issues.json -v
   ```

2. **Open Copilot Chat** and ask:
   ```
   @workspace I have sonar-issues.json with SonarCloud findings. 
   Please analyze the top 10 issues by severity and suggest fixes.
   ```

3. **For specific files:**
   ```
   @workspace In sonar-issues.json, show me all issues in src/main.rs 
   and help me fix the cognitive complexity problems.
   ```

4. **For specific issue types:**
   ```
   @workspace Show me all VULNERABILITY issues from sonar-issues.json 
   and provide code fixes.
   ```

### Workflow in Pull Requests

For PRs, you can integrate this into your review process:

```bash
# In your PR CI or locally
python3 sonar.py --pull-request $PR_NUMBER -o pr-issues.json -v

# Review with Copilot
# Copilot can then suggest fixes for new issues introduced in the PR
```

## Advanced Features

### Staleness Detection

The script automatically checks the age of the SonarCloud analysis:

```bash
python3 sonar.py --branch main --max-age-hours 6 -v
```

If analysis is older than the threshold, you'll see:
```
⚠️  WARNING: Analysis is 8.5 hours old (threshold: 6h)
    Results may be stale. Consider triggering a new analysis.
```

### Multiple Output Formats

```bash
# Copilot-optimized format (default)
python3 sonar.py --branch main --format copilot -o issues.json

# Raw SonarCloud API format
python3 sonar.py --branch main --format json -o raw-issues.json

# SARIF format (coming soon)
# python3 sonar.py --branch main --format sarif -o issues.sarif
```

### Combining with CI/CD

Example GitHub Actions workflow:

```yaml
- name: Fetch SonarCloud Issues
  env:
    SONAR_TOKEN: ${{ secrets.SONAR_TOKEN }}
    SONAR_ORG: edgefirstai
    SONAR_PROJECT: EdgeFirstAI_client
  run: |
    python3 sonar.py --branch ${{ github.ref_name }} \
      --severity BLOCKER,CRITICAL \
      --output sonar-issues.json \
      --verbose

- name: Upload Issues Artifact
  uses: actions/upload-artifact@v4
  with:
    name: sonar-issues
    path: sonar-issues.json
```

## Migration from Old Script

The new `sonar.py` replaces the previous version with backwards-compatible environment variables:

### Old usage:
```bash
export SONAR_HOST_URL=https://sonarcloud.io
export SONAR_TOKEN=<token>
export SONAR_ORG=edgefirstai
export SONAR_PROJECT=EdgeFirstAI_client
export BRANCH=main
export REPORT_PATH=sonar-report.json

python3 sonar.py
```

### New equivalent:
```bash
export SONAR_TOKEN=<token>
export SONAR_ORG=edgefirstai
export SONAR_PROJECT=EdgeFirstAI_client

python3 sonar.py --branch main --output sonar-report.json
```

### Key differences:

1. ✅ **Explicit branch/PR**: Must specify `--branch` or `--pull-request`
2. ✅ **Better filtering**: Can filter by severity, type, status
3. ✅ **Staleness detection**: Warns about outdated analyses
4. ✅ **Copilot-optimized output**: Structured for IDE consumption
5. ✅ **Verbose mode**: Use `-v` for detailed progress

## Troubleshooting

### Authentication errors

```
❌ API Error: 401 Client Error: Unauthorized
```

**Solution:** Check your `SONAR_TOKEN` is valid and has read permissions.

### Stale results

```
⚠️  WARNING: Analysis is 25.3 hours old (threshold: 24h)
```

**Solution:** Trigger a new SonarCloud analysis:
- Push a new commit, or
- Manually trigger analysis via SonarCloud dashboard, or
- Wait for scheduled analysis to run

### No issues found

```
Found 0 issues
Found 0 hotspots
```

**Possible causes:**
- Wrong branch name
- No analysis exists for this branch yet
- All issues have been resolved
- Filters are too restrictive

**Solution:** Try with `--verbose` to see API calls, or check the SonarCloud dashboard.

### Rate limiting

```
❌ API Error: 429 Client Error: Too Many Requests
```

**Solution:** Wait a few minutes and retry. SonarCloud has rate limits on API calls.

## Tips for Best Results

1. **Run fresh analyses regularly** - Don't rely on old data
2. **Start with high-severity issues** - Use `--severity BLOCKER,CRITICAL`
3. **Focus on specific types** - Use `--type BUG,VULNERABILITY` for security
4. **Use verbose mode** - Add `-v` to understand what's happening
5. **Combine with Copilot** - Ask specific questions about the JSON output
6. **Iterate quickly** - Fix issues, commit, wait for new analysis, repeat

## Support

For issues or questions:
- Check the [CONTRIBUTING.md](CONTRIBUTING.md) guide
- Open an issue on GitHub
- Review [SonarCloud API documentation](https://sonarcloud.io/web_api)

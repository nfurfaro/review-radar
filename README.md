# Review Radar (rr)

A simple CLI tool to find GitHub PRs where your review has been requested across multiple organizations.

## Features

- 🔍 Search for PRs where your review is requested
- 👤 Find your own open PRs
- 🏢 Support for multiple GitHub organizations
- 🔧 Configurable repository filtering with regex patterns
- ⚡ Fast parallel searches across organizations

## Installation

```bash
# Install globally using cargo
cargo install --path .
```

This installs the `rr` command to your cargo bin directory (usually `~/.cargo/bin/`), which should be in your PATH.

## Setup

```bash
# First, authenticate with GitHub CLI (if not already done)
gh auth login

# Initialize your configuration with one or more organizations
rr init "org1,org2,org3" your-username

# Or initialize with a single organization
rr init my-org your-username

# Optionally add a repository filter pattern during init
rr init my-org your-username -r "backend-.*"
```

## Usage

### Basic Commands

```bash
# Find all PRs where your review is requested (across all configured orgs)
rr

# Find your own open PRs instead of review requests
rr --own

# Show current configuration
rr config
```

### Organization Management

```bash
# Add a new organization to your config
rr set --orgs +new-org

# Remove an organization from your config
rr set --orgs -old-org

# Replace all organizations
rr set --orgs "org1,org2,org3"

# Override organizations for a single search (doesn't save to config)
rr --orgs "temp-org1,temp-org2"
```

### Repository Filtering

```bash
# Set a repository filter pattern (regex)
rr set -r "backend-.*"

# Clear the repository filter
rr set -r none

# Override the filter for a single search
rr -r "frontend-.*"

# Search with both org override and repo filter
rr --orgs "my-org" -r "api-.*"
```

### Command Overrides

```bash
# Override username for a specific search
rr --username different-username

# Combine multiple overrides
rr --orgs "temp-org" --username "temp-user" -r "test-.*" --own
```

## Configuration

Your configuration is stored at `~/.config/review-radar/config.toml` and includes:

- **Organizations**: List of GitHub organizations to search
- **Username**: Your GitHub username
- **Repository Pattern**: Optional regex to filter repository names

### Example Configuration

```toml
orgs = ["my-company", "open-source-org", "side-project-org"]
username = "myusername"
repo_pattern = "backend-.*"
```

## Command Reference

### Main Commands

- `rr` - Search for PRs requesting your review
- `rr --own` / `rr -o` - Search for your own open PRs
- `rr init <orgs> <username>` - Initialize configuration
- `rr set` - Update configuration
- `rr config` - Show current configuration

### Flags and Options

- `--orgs <ORGS>` - Override configured organizations (comma-separated)
- `--username <USERNAME>` / `-u <USERNAME>` - Override configured username
- `--own` / `-o` - Show your own open PRs instead of review requests
- `--repo-pattern <PATTERN>` / `-r <PATTERN>` - Regex pattern to filter repositories

### Organization Management in `rr set`

- `--orgs "org1,org2"` - Replace all organizations
- `--orgs +new-org` - Add an organization
- `--orgs -old-org` - Remove an organization

## Examples

```bash
# Setup for multiple organizations
rr init "acme-corp,open-source-foundation" john.doe

# Add a repository filter for backend services only
rr set -r "^(api|backend|service)-.*"

# Find your review requests across all configured orgs
rr

# Find your own PRs in just one organization
rr --orgs acme-corp --own

# Temporarily search a different org with a specific pattern
rr --orgs temp-org -r "frontend-.*"

# Add a new organization to your config
rr set --orgs +new-startup

# Remove an organization you no longer work with
rr set --orgs -old-company
```

## Requirements

- GitHub CLI (`gh`) must be installed and authenticated
- Rust and Cargo for installation
- Access to the GitHub organizations you want to search

## Output

The tool provides colored output showing:
- 🔗 PR number and title
- 👤 Author information
- 🌐 Direct URL to the PR
- Progress indicators during multi-organization searches

Example output:
```
🔍 Searching for PRs in 3 organizations where john.doe has been requested for review...
🏛️  Found 25 total repositories across 3 organization(s)
🔍 Checked 25 repositories

📋 Found 3 PR(s) requesting your review:

🔗 #123 - Add user authentication system
   👤 Author: alice.smith
   🌐 URL: https://github.com/acme-corp/backend-api/pull/123

🔗 #456 - Update documentation for new API
   👤 Author: bob.jones
   🌐 URL: https://github.com/open-source-foundation/docs/pull/456

🔗 #789 - Fix memory leak in worker process
   👤 Author: charlie.brown
   🌐 URL: https://github.com/acme-corp/worker-service/pull/789
```
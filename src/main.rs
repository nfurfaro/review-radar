use anyhow::Result;
use clap::{Parser, Subcommand};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser, Debug)]
#[command(name = "review-radar")]
#[command(about = "Find GitHub PRs where your review has been requested")]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(long, help = "Override configured organization(s), comma-separated")]
    orgs: Option<String>,

    #[arg(short, long, help = "Override configured username")]
    username: Option<String>,

    #[arg(
        short = 'o',
        long = "own",
        help = "Show your own open PRs instead of review requests"
    )]
    own_prs: bool,

    #[arg(
        short = 'r',
        long = "repo-pattern",
        help = "Regex pattern to filter repository names (e.g., 'void-.*')"
    )]
    repo_pattern: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Initialize configuration")]
    Init {
        #[arg(help = "GitHub organization(s), comma-separated")]
        orgs: String,
        #[arg(help = "Your GitHub username")]
        username: String,
        #[arg(
            short = 'r',
            long = "repo-pattern",
            help = "Regex pattern to filter repository names"
        )]
        repo_pattern: Option<String>,
    },
    #[command(about = "Update configuration")]
    Set {
        #[arg(
            long,
            help = "GitHub organization(s), comma-separated (use '+org' to add, '-org' to remove)"
        )]
        orgs: Option<String>,
        #[arg(long, help = "Your GitHub username")]
        username: Option<String>,
        #[arg(
            short = 'r',
            long = "repo-pattern",
            help = "Regex pattern to filter repository names (use 'none' to clear)"
        )]
        repo_pattern: Option<String>,
    },
    #[command(about = "Show current configuration")]
    Config,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    orgs: Vec<String>,
    username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    repo_pattern: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PullRequest {
    number: u32,
    title: String,
    html_url: String,
    user: User,
}

#[derive(Debug, Deserialize)]
struct User {
    login: String,
}

impl Config {
    fn config_path() -> Result<PathBuf> {
        let config_dir =
            dirs::config_dir().ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        Ok(config_dir.join("review-radar").join("config.toml"))
    }

    fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Err(anyhow::anyhow!(
                "Configuration not found. Run 'review-radar init <orgs> <username>' to set up."
            ));
        }
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

struct GitHubClient;

impl GitHubClient {
    fn new() -> Self {
        Self
    }

    fn search_prs_for_user(
        &self,
        orgs: &[String],
        username: &str,
        repo_pattern: Option<&str>,
    ) -> Result<Vec<PullRequest>> {
        self.search_prs(orgs, username, false, repo_pattern)
    }

    fn search_own_prs(
        &self,
        orgs: &[String],
        username: &str,
        repo_pattern: Option<&str>,
    ) -> Result<Vec<PullRequest>> {
        self.search_prs(orgs, username, true, repo_pattern)
    }

    fn search_prs(
        &self,
        orgs: &[String],
        username: &str,
        own_prs: bool,
        repo_pattern: Option<&str>,
    ) -> Result<Vec<PullRequest>> {
        let mut all_repos = Vec::new();
        let total_orgs = orgs.len();

        println!(
            "📡 Getting repositories from {} organization(s)...",
            total_orgs
        );

        for (idx, org) in orgs.iter().enumerate() {
            print!(
                "\r🏛️  Fetching from {} ({}/{})...",
                org,
                idx + 1,
                total_orgs
            );
            std::io::stdout().flush().unwrap();

            let repos_output = Command::new("gh")
                .args(["repo", "list", org, "--json", "name", "--limit", "1000"])
                .output()?;

            if !repos_output.status.success() {
                eprintln!("\n⚠️  Failed to list repositories for {}, skipping...", org);
                continue;
            }

            let repos_stdout = String::from_utf8(repos_output.stdout)?;
            let mut org_repos: Vec<GhRepo> = serde_json::from_str(&repos_stdout)?;

            // Add org name to each repo for later reference
            for repo in &mut org_repos {
                repo.org = org.clone();
            }
            all_repos.extend(org_repos);
        }

        println!(
            "\r🏛️  Found {} total repositories across {} organization(s)",
            all_repos.len(),
            total_orgs
        );

        let repos = all_repos;

        // Filter repositories if pattern is provided
        let filtered_repos = if let Some(pattern) = repo_pattern {
            let regex = Regex::new(pattern)
                .map_err(|e| anyhow::anyhow!("Invalid regex pattern '{}': {}", pattern, e))?;

            // Only keep repos that match the pattern
            let matching: Vec<GhRepo> = repos
                .into_iter()
                .filter(|repo| regex.is_match(&repo.name))
                .collect();

            println!(
                " found {} repositories matching pattern '{}'",
                matching.len(),
                pattern
            );
            matching
        } else {
            println!(" found {} repositories", repos.len());
            repos
        };

        let mut all_prs = Vec::new();
        let mut checked_repos = 0;
        let total_repos = filtered_repos.len();

        // For each repository, get PRs
        for repo in filtered_repos {
            checked_repos += 1;
            if checked_repos % 10 == 0 || checked_repos == 1 {
                print!(
                    "\r🔍 Checking repositories... {}/{}",
                    checked_repos, total_repos
                );
                std::io::stdout().flush().unwrap();
            }

            let repo_name = format!("{}/{}", repo.org, repo.name);

            let mut args = vec![
                "pr",
                "list",
                "--repo",
                &repo_name,
                "--json",
                "number,title,url,author,reviewRequests",
                "--state",
                "open",
            ];

            if own_prs {
                args.extend(&["--author", username]);
            }

            let prs_output = Command::new("gh").args(&args).output()?;

            if !prs_output.status.success() {
                // Skip repos we can't access instead of failing
                continue;
            }

            let prs_stdout = String::from_utf8(prs_output.stdout)?;
            let prs: Vec<GhPullRequest> = serde_json::from_str(&prs_stdout).unwrap_or_default();

            for pr in prs {
                if own_prs {
                    // For own PRs, just add all PRs by the user
                    all_prs.push(PullRequest {
                        number: pr.number,
                        title: pr.title,
                        html_url: pr.url,
                        user: User {
                            login: pr.author.login,
                        },
                    });
                } else {
                    // For review requests, filter PRs where the user is requested for review
                    let is_requested = pr.review_requests.iter().any(|req| req.login == username);
                    if is_requested {
                        all_prs.push(PullRequest {
                            number: pr.number,
                            title: pr.title,
                            html_url: pr.url,
                            user: User {
                                login: pr.author.login,
                            },
                            });
                    }
                }
            }
        }

        print!("\r🔍 Checked {} repositories            \n", checked_repos);

        Ok(all_prs)
    }
}

#[derive(Debug, Deserialize)]
struct GhRepo {
    name: String,
    #[serde(skip)]
    org: String,
}

#[derive(Debug, Deserialize)]
struct GhPullRequest {
    number: u32,
    title: String,
    url: String,
    author: GhUser,
    #[serde(rename = "reviewRequests")]
    review_requests: Vec<GhUser>,
}

#[derive(Debug, Deserialize)]
struct GhUser {
    login: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Some(Commands::Init {
            orgs,
            username,
            repo_pattern,
        }) => {
            let org_list: Vec<String> = orgs.split(',').map(|s| s.trim().to_string()).collect();
            let config = Config {
                orgs: org_list.clone(),
                username,
                repo_pattern,
            };
            config.save()?;
            println!("✅ Configuration saved successfully!");
            println!("📋 Organizations: {}", org_list.join(", "));
            if config.repo_pattern.is_some() {
                println!(
                    "📋 Repository filter pattern: {}",
                    config.repo_pattern.as_ref().unwrap()
                );
            }
            println!("💡 Make sure you're authenticated with GitHub CLI: gh auth status");
            return Ok(());
        }
        Some(Commands::Set {
            orgs,
            username,
            repo_pattern,
        }) => {
            let mut config = Config::load()?;
            let mut updated = false;

            if let Some(org_str) = orgs {
                if let Some(stripped) = org_str.strip_prefix('+') {
                    // Add organization
                    let new_org = stripped.trim().to_string();
                    if !config.orgs.contains(&new_org) {
                        config.orgs.push(new_org.clone());
                        println!("➕ Added organization: {}", new_org);
                        updated = true;
                    } else {
                        println!("ℹ️  Organization '{}' already exists", new_org);
                    }
                } else if let Some(stripped) = org_str.strip_prefix('-') {
                    // Remove organization
                    let remove_org = stripped.trim().to_string();
                    if let Some(pos) = config.orgs.iter().position(|x| x == &remove_org) {
                        config.orgs.remove(pos);
                        println!("➖ Removed organization: {}", remove_org);
                        updated = true;
                    } else {
                        println!("ℹ️  Organization '{}' not found", remove_org);
                    }
                } else {
                    // Replace all organizations
                    config.orgs = org_str.split(',').map(|s| s.trim().to_string()).collect();
                    println!("✅ Updated organizations");
                    updated = true;
                }
            }
            if let Some(new_username) = username {
                config.username = new_username;
                updated = true;
            }
            if let Some(new_pattern) = repo_pattern {
                if new_pattern.to_lowercase() == "none" {
                    config.repo_pattern = None;
                    println!("🗑️  Cleared repository filter pattern");
                } else {
                    // Validate the regex
                    match Regex::new(&new_pattern) {
                        Ok(_) => {
                            config.repo_pattern = Some(new_pattern);
                            println!("✅ Updated repository filter pattern");
                        }
                        Err(e) => {
                            println!("❌ Invalid regex pattern: {}", e);
                            return Ok(());
                        }
                    }
                }
                updated = true;
            }

            if updated {
                config.save()?;
                println!("✅ Configuration updated successfully!");
            } else {
                println!("ℹ️  No changes specified");
            }
            return Ok(());
        }
        Some(Commands::Config) => {
            match Config::load() {
                Ok(config) => {
                    println!("Current configuration:");
                    println!("  Organizations: {}", config.orgs.join(", "));
                    println!("  Username: {}", config.username);
                    if let Some(pattern) = &config.repo_pattern {
                        println!("  Repository filter: {}", pattern);
                    } else {
                        println!("  Repository filter: (none)");
                    }

                    // Check gh auth status
                    let output = Command::new("gh").args(["auth", "status"]).output();
                    match output {
                        Ok(output) if output.status.success() => {
                            println!("  GitHub CLI: ✅ Authenticated");
                        }
                        _ => {
                            println!("  GitHub CLI: ❌ Not authenticated (run 'gh auth login')");
                        }
                    }
                }
                Err(e) => {
                    println!("❌ {}", e);
                }
            }
            return Ok(());
        }
        None => {}
    }

    // Check if gh is authenticated before proceeding
    let auth_output = Command::new("gh").args(["auth", "status"]).output()?;
    if !auth_output.status.success() {
        println!("❌ GitHub CLI is not authenticated. Run 'gh auth login' first.");
        return Ok(());
    }

    let config = Config::load()?;

    // Use command-line orgs if provided, otherwise use config orgs
    let orgs = if let Some(org_str) = args.orgs {
        org_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        config.orgs.clone()
    };

    if orgs.is_empty() {
        return Err(anyhow::anyhow!(
            "No organizations configured. Use 'rr init' or 'rr set --orgs' to configure."
        ));
    }

    let username = args.username.as_ref().unwrap_or(&config.username);

    let client = GitHubClient::new();

    // Use command-line pattern if provided, otherwise use config pattern
    let repo_pattern = args
        .repo_pattern
        .as_deref()
        .or(config.repo_pattern.as_deref());

    let (prs, search_type) = if args.own_prs {
        let org_list = if orgs.len() > 2 {
            format!("{} organizations", orgs.len())
        } else {
            orgs.join(", ")
        };
        let search_desc = if let Some(pattern) = repo_pattern {
            format!(
                "🔍 Searching for {}'s open PRs in {} (repos matching '{}')...",
                username, org_list, pattern
            )
        } else {
            format!(
                "🔍 Searching for {}'s open PRs in {}...",
                username, org_list
            )
        };
        println!("{}", search_desc);
        let prs = client.search_own_prs(&orgs, username, repo_pattern)?;
        (prs, "you have open")
    } else {
        let org_list = if orgs.len() > 2 {
            format!("{} organizations", orgs.len())
        } else {
            orgs.join(", ")
        };
        let search_desc = if let Some(pattern) = repo_pattern {
            format!("🔍 Searching for PRs in {} where {} has been requested for review (repos matching '{}')...", org_list, username, pattern)
        } else {
            format!(
                "🔍 Searching for PRs in {} where {} has been requested for review...",
                org_list, username
            )
        };
        println!("{}", search_desc);
        let prs = client.search_prs_for_user(&orgs, username, repo_pattern)?;
        (prs, "requesting your review")
    };

    if prs.is_empty() {
        if args.own_prs {
            println!("✅ No open PRs found by you!");
        } else {
            println!("✅ No PRs found where your review has been requested!");
        }
        return Ok(());
    }

    println!("\n📋 Found {} PR(s) {}:\n", prs.len(), search_type);

    for pr in prs {
        println!("🔗 #{} - {}", pr.number, pr.title);
        println!("   👤 Author: {}", pr.user.login);
        println!("   🌐 URL: {}", pr.html_url);
        println!();
    }

    Ok(())
}

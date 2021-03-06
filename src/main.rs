use anyhow::{bail, Context, Result};
use futures::executor::block_on;
use serde::Deserialize;
use std::{env, path::Path, process::Command};

use action_label_rust_incompatible as lib;

/*
Example:
{
  "old_version": "0.1.0",
  "new_version": "0.1.1",
  "changes": {
    "path_changes": [],
    "changes": [
      {
        "name": "testa",
        "max_category": "Breaking",
        "new_span": {
          "file": "/tmp/testdir/head/src/lib.rs",
          "line_lo": 1,
          "line_hi": 1,
          "col_lo": 0,
          "col_hi": 22
        },
        "changes": [
          [
            "type error: incorrect number of function parameters",
            null
          ]
        ]
      }
    ],
    "max_category": "Breaking"
  }
}
*/

#[derive(Debug, Deserialize)]
enum ChangeTypes {
    Patch,
    NonBreaking,
    TechnicallyBreaking,
    Breaking,
}

#[derive(Debug, Deserialize)]
struct SemverResultChanges {
    max_category: ChangeTypes,
    path_changes: serde_json::Value,
    changes: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct SemverResult {
    old_version: String,
    new_version: String,
    changes: SemverResultChanges,
}

fn main() -> Result<()> {
    // Get environment arguments
    let dnf_deps = env::var("INPUT_DNF-DEPENDENCIES").ok();
    let github_token = env::var("INPUT_REPO-TOKEN").expect("No repo token provided");
    let head_sha = env::var("GITHUB_SHA").expect("No head sha specified");
    let head_ref = env::var("GITHUB_REF").expect("No head ref specified");
    let head_ref_parts: Vec<&str> = head_ref.split('/').collect();
    // TODO: expect() for PRs
    let base_ref = env::var("GITHUB_BASE_REF").expect("No base ref specified");
    let workspace = env::var("GITHUB_WORKSPACE").expect("No workspace provided");
    let repo = env::var("GITHUB_REPOSITORY").expect("No repository provided");
    let gh_server = env::var("GITHUB_SERVER_URL").expect("No GitHub server URL provided");

    // Get label configurations
    let ct_label_patch = env::var("INPUT_LABEL-PATCH").ok();
    let ct_label_non_breaking = env::var("INPUT_LABEL-NON-BREAKING").ok();
    let ct_label_technically_breaking = env::var("INPUT_LABEL-TECHNICALLY-BREAKING").ok();
    let ct_label_breaking = env::var("INPUT_LABEL-BREAKING").ok();

    let mut base_ref = base_ref;
    if base_ref.is_empty() {
        println!("No base ref, assuming refs/heads/main");
        base_ref = "refs/heads/main".to_string()
    }
    let base_ref = base_ref;

    println!("dnf_deps: {:?}", &dnf_deps);
    // No, we are not printing github_token.... Nice try.
    println!("head_sha: {}", &head_sha);
    println!("head_ref: {}", &head_ref);
    println!("head_ref_parts: {:?}", &head_ref_parts);
    println!("base_ref: {}", &base_ref);
    println!("workspace: {}", &workspace);
    println!("repo: {}", &repo);
    println!("gh_server: {}", &gh_server);

    // Labels
    println!("Label: Patch: {:?}", ct_label_patch);
    println!("Label: Non-Breaking: {:?}", ct_label_non_breaking);
    println!(
        "Label: Technically-Breaking: {:?}",
        ct_label_technically_breaking
    );
    println!("Label: Breaking: {:?}", ct_label_breaking);

    // Compute PR number
    // Example: refs/pull/42/merge
    let pr_num = if head_ref_parts.len() == 4
        && head_ref_parts[0] == "refs"
        && head_ref_parts[1] == "pull"
        && head_ref_parts[3] == "merge"
    {
        Some(
            head_ref_parts[2]
                .parse::<u64>()
                .with_context(|| format!("Error parsing PR number: {}", head_ref_parts[2]))?,
        )
    } else {
        None
    };
    println!("PR Number: {:?}", pr_num);

    // Compute owner/repo
    let repo: Vec<&str> = repo.split('/').collect();
    if repo.len() != 2 {
        bail!("Invalid repository name: {:?}", repo);
    }
    let owner = repo[0];
    let repo = repo[1];

    // Run DNF install
    if let Some(deps) = dnf_deps {
        let deps = deps.split(' ');
        if !Command::new("dnf")
            .arg("install")
            .arg("-y")
            .args(deps)
            .status()
            .context("Error running DNF install")?
            .success()
        {
            bail!("Error installing DNF dependencies");
        }
    }

    // Initialize OctoCrab
    let oc = octocrab::OctocrabBuilder::new()
        .base_url(&gh_server)
        .context("Error setting github base url")?
        .personal_token(github_token)
        .build()
        .context("Error building github client")?;
    let oc_issues = oc.issues(owner, repo);

    // Run analysis
    let changetype =
        run(&head_sha, &head_ref, &base_ref, &workspace).context("Error running analysis")?;

    let set_label = match changetype {
        ChangeTypes::Patch => &ct_label_patch,
        ChangeTypes::NonBreaking => &ct_label_non_breaking,
        ChangeTypes::TechnicallyBreaking => &ct_label_technically_breaking,
        ChangeTypes::Breaking => &ct_label_breaking,
    };
    let remove_labels = match changetype {
        ChangeTypes::Patch => vec![
            &ct_label_non_breaking,
            &ct_label_technically_breaking,
            &ct_label_breaking,
        ],
        ChangeTypes::NonBreaking => vec![
            &ct_label_patch,
            &ct_label_technically_breaking,
            &ct_label_breaking,
        ],
        ChangeTypes::TechnicallyBreaking => {
            vec![&ct_label_patch, &ct_label_non_breaking, &ct_label_breaking]
        }
        ChangeTypes::Breaking => vec![
            &ct_label_patch,
            &ct_label_non_breaking,
            &ct_label_technically_breaking,
        ],
    };

    if pr_num.is_none() {
        println!(
            "Non-PR ({}), so won't label with status: {:?}",
            &head_ref, changetype
        );
        println!("Label we would have applied: {:?}", set_label);
        println!("Labels we would have removed: {:?}", remove_labels);
        return Ok(());
    }
    // Checked just above
    let pr_num = pr_num.unwrap();

    block_on(lib::set_and_remove_labels(
        oc_issues,
        pr_num,
        set_label,
        remove_labels,
    ))
    .context("Error setting and removing labels")?;

    Ok(())
}

fn run(head_sha: &str, head_ref: &str, base_ref: &str, workspace: &str) -> Result<ChangeTypes> {
    let temp_dir =
        tempdir::TempDir::new("analyzer_work_dir").context("Error creating temporary directory")?;

    let base_sha = determine_base_sha(workspace, base_ref, base_ref != head_ref)
        .context("Error determining base SHA")?;
    println!("Base sha: {}", &base_sha);

    println!(
        "Comparing {}..{} ({} -> {})",
        &base_sha, &head_sha, &base_ref, &head_ref
    );

    let (base_dir, head_dir) =
        lib::prepare_directories(workspace, temp_dir.path(), &base_sha, head_sha)
            .context("Error preparing work directories")?;

    let analysis = run_analysis(&base_dir, &head_dir).context("Error running analysis")?;

    println!("Unparsed result: {:?}", String::from_utf8_lossy(&analysis));

    let analysis_result: SemverResult =
        serde_json::from_slice(&analysis).context("Error parsing analysis result")?;

    println!("Full analysis result: {:?}", analysis_result);

    Ok(analysis_result.changes.max_category)
}

fn determine_base_sha(workspace: &str, base_ref: &str, do_fetch: bool) -> Result<String> {
    if do_fetch
        && !Command::new("git")
            .arg("-C")
            .arg(&workspace)
            .arg("fetch")
            .arg("--depth")
            .arg("1")
            .arg("origin")
            .arg(format!("{}:{}", &base_ref, &base_ref))
            .status()
            .context("Error running pull")?
            .success()
    {
        bail!("Was unable to pull base ref");
    }

    let ref_out = Command::new("git")
        .arg("-C")
        .arg(&workspace)
        .arg("show-ref")
        .arg("--hash")
        .arg(&base_ref)
        .output()
        .context("Error running rev-parse")?;
    println!("Ref out: {:?}", String::from_utf8_lossy(&ref_out.stdout));
    println!("Ref err: {:?}", String::from_utf8_lossy(&ref_out.stderr));
    let base_sha = String::from_utf8(ref_out.stdout).context("Error parsing base sha")?;
    Ok(base_sha.trim().to_string())
}

fn run_analysis(base_dir: &Path, head_dir: &Path) -> Result<Vec<u8>> {
    let toolchain_version = std::fs::read_to_string("/tmp/toolchain_version")
        .context("Error reading toolchain version to use")?;
    let toolchain_version = toolchain_version.trim();

    // Run analysis
    println!("Running analysis");
    let out = Command::new("/analysis_cargo/bin/cargo")
        .env("CARGO_HOME", "/analysis_cargo")
        .env("RUSTUP_HOME", "/analysis_rustup")
        .arg(format!("+{}", toolchain_version))
        .arg("semver")
        .arg("--json")
        .arg("--all-features")
        .arg("--stable-path")
        .arg(base_dir.join("Cargo.toml"))
        .arg("--current-path")
        .arg(head_dir.join("Cargo.toml"))
        .output()
        .context("Error running analysis")?;
    println!("Analysis stdout: {}", String::from_utf8_lossy(&out.stdout));
    println!("Analysis stderr: {}", String::from_utf8_lossy(&out.stderr));
    Ok(out.stdout)
}

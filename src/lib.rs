use anyhow::{bail, Context, Result};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub async fn set_and_remove_labels(
    oc_issues: octocrab::issues::IssueHandler<'_>,
    pr_num: u64,
    set_label: &Option<String>,
    remove_labels: Vec<&Option<String>>,
) -> Result<()> {
    if let Some(label) = set_label {
        oc_issues
            .add_labels(pr_num, &[label.clone()])
            .await
            .context("Error setting new label")?;
    }

    for label in remove_labels {
        if let Some(label) = label {
            oc_issues
                .remove_label(pr_num, label)
                .await
                .context("Error removing old labels")?;
        }
    }

    Ok(())
}

pub fn prepare_directories(
    workspace: &str,
    temp_dir: &Path,
    base_sha: &str,
    head_sha: &str,
) -> Result<(PathBuf, PathBuf)> {
    // Copy the code twice
    let base_dir = temp_dir.join("base");
    let head_dir = temp_dir.join("head");

    // Ensure correct repos are checked out
    println!("Preparing base clone");
    copy_dir::copy_dir(&workspace, &base_dir).context("Error copying base clone")?;
    if !Command::new("git")
        .arg("-C")
        .arg(&base_dir)
        .arg("checkout")
        .arg("--detach")
        .arg("--force")
        .arg(&base_sha)
        .status()
        .context("Error checking out base sha in base dir")?
        .success()
    {
        bail!("Error checking out base sha");
    }

    println!("Preparing head clone");
    copy_dir::copy_dir(&workspace, &head_dir).context("Error copying head clone")?;
    if !Command::new("git")
        .arg("-C")
        .arg(&head_dir)
        .arg("checkout")
        .arg("--detach")
        .arg("--force")
        .arg(&head_sha)
        .status()
        .context("Error checking out head sha in head dir")?
        .success()
    {
        bail!("Error checking out head sha");
    }

    Ok((base_dir, head_dir))
}

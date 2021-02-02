# action-label-rust-incompatible
This action can tag PRs with information on whether the PR includes different types of changes.
(patch, non-breaking, technically-breaking, breaking)

## Inputs

### `repo-token`
**Required** The repository token to use.

### `repo-base-ref`
The base ref to use if none was specified (i.e. during push), default: refs/heads/main.

### `label-patch`
The label to apply to PRs with Patch changes.

### `label-non-breaking`
The label to apply to PRs with Non-breaking changes.

### `label-technically-breaking`
The label to apply to PRs with Technically Breaking changes.

### `label-breaking`
The label to apply to PRs with Breaking changes.

### `dnf-dependencies`
A space-separated list of DNF packages to install before running.

## Example usage

```
- uses: puiterwijk/action-label-rust-incompatible@v1
  with:
    repo-token: '${{ secrets.GITHUB_TOKEN }}'
    label-patch: "API/Patch"
    label-non-breaking: "API/Non-Breaking"
    label-technically-breaking: "API/Technically-Breaking"
    label-breaking: "API/Breaking"
    dnf-dependencies: 'dnf'
```

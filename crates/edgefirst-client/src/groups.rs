// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

//! Types for dataset group (train/val/test) management.

use crate::{Error, SampleID};
use serde::{Deserialize, Serialize};

/// Per-group sample counts within a [`GroupSummary`].
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct GroupCount {
    /// The group ID.
    pub id: u64,
    /// The group name.
    pub name: String,
    /// Number of images assigned to the group.
    pub images: u64,
    /// Number of distinct sequences with at least one image in the group.
    pub sequences: u64,
}

/// Group membership counts for a dataset.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct GroupSummary {
    /// Total number of images in the dataset.
    pub total: u64,
    /// Number of images that belong to no group.
    pub ungrouped: u64,
    /// Counts for each group in the dataset.
    pub groups: Vec<GroupCount>,
    /// Number of sequences whose images are spread over more than one group.
    pub mixed_sequences: u64,
}

/// A target group and the share of samples it should receive in a split.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct SplitTarget {
    /// The group name. A missing group is created when a non-dry-run split
    /// moves at least one image overall, even if this target gets none.
    pub name: String,
    /// The share of samples for the group, in percent. The server rejects a
    /// request in which every percentage is zero.
    pub percent: f64,
}

/// Which samples a split draws from.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SplitScope {
    /// Only samples that belong to no group.
    Ungrouped,
    /// Samples currently in the named groups, optionally with the ungrouped
    /// samples as well.
    Groups {
        /// Names of the groups to redistribute.
        from: Vec<String>,
        /// Also include samples that belong to no group.
        include_ungrouped: bool,
    },
    /// Every sample in the dataset.
    All,
}

/// The outcome for one target group of a split.
///
/// `id` is `None` when the group does not exist, which happens only for a dry
/// run or a split that moved no image at all. `created` is true only when
/// this call created the group.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SplitGroupResult {
    /// The group ID, if the group exists.
    pub id: Option<u64>,
    /// The group name.
    pub name: String,
    /// Whether this call created the group.
    pub created: bool,
    /// The requested share, as a percentage of the split samples.
    pub target_percent: f64,
    /// The share actually achieved, as a percentage of the split samples.
    pub achieved_percent: f64,
    /// Number of images assigned to the group by the split.
    pub images: u64,
    /// Number of sequences assigned to the group by the split.
    pub sequences: u64,
}

/// The result of a [`Client::split_groups`](crate::Client::split_groups) call.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SplitReport {
    /// The seed used; pass it back to reproduce the same assignment.
    pub seed: String,
    /// The scope the server applied.
    pub scope: String,
    /// Target groups in request order, followed by any non-target group that
    /// ungrouped frames joined.
    pub groups: Vec<SplitGroupResult>,
    /// Number of images left without a group.
    pub ungrouped: u64,
    /// Number of sequences skipped because their images already span several
    /// groups.
    pub skipped_mixed_sequences: u64,
}

/// The maximum number of ids the server accepts in one `samples.set_group`
/// call. [`Client::set_samples_group`](crate::Client::set_samples_group)
/// splits larger selections into requests of at most this size.
pub const SET_GROUP_MAX_IDS: usize = 100_000;

/// The samples a [`Client::set_samples_group`](crate::Client::set_samples_group)
/// call moves.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SampleSelection {
    /// Individual samples.
    Samples(Vec<SampleID>),
    /// Every sample of the given sequences, identified by sequence UUID.
    Sequences(Vec<String>),
}

/// Rewrites the server's "unknown method" reply for `method` into an error
/// telling the user the Studio server predates the endpoint. Other errors pass
/// through unchanged.
pub(crate) fn map_unsupported(method: &str, err: Error) -> Error {
    match err {
        Error::RpcError(10, msg) if msg.starts_with("Invalid Method") => Error::RpcError(
            10,
            format!("{method} is not supported by this Studio server; it needs updating"),
        ),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_unsupported_rewrites_invalid_method() {
        let err = map_unsupported(
            "groups.split",
            Error::RpcError(10, "Invalid Method 'x'".into()),
        );
        assert!(matches!(
            err,
            Error::RpcError(10, m) if m == "groups.split is not supported by this Studio server; it needs updating"
        ));
    }

    #[test]
    fn map_unsupported_leaves_other_errors_unchanged() {
        let err = map_unsupported(
            "groups.split",
            Error::RpcError(10, "Permission denied".into()),
        );
        assert!(matches!(err, Error::RpcError(10, m) if m == "Permission denied"));

        let err = map_unsupported("groups.split", Error::RpcError(101, "not found".into()));
        assert!(matches!(err, Error::RpcError(101, m) if m == "not found"));
    }
}

//! Each request in words for the status bar, from the owner's side: what Taskboard is doing for them.

use super::endpoint::Endpoint;

impl Endpoint {
    pub fn doing(&self) -> &'static str {
        match self {
            Self::Myself | Self::TenantInfo => "Checking the connection",
            Self::Fields
            | Self::Statuses
            | Self::ProjectSearch { .. }
            | Self::ProjectVersions { .. }
            | Self::MyPermissions { .. } => "Reading Jira's setup",
            Self::SearchJql { .. } | Self::BulkFetch { .. } | Self::ApproximateCount { .. } => {
                "Fetching issues"
            }
            Self::IssueComments { .. }
            | Self::IssueRemoteLinks { .. }
            | Self::IssueChangelog { .. } => "Loading the issue's details",
            Self::AttachmentContent { .. } => "Downloading an attachment",
            Self::JqlParse { .. } => "Checking the query",
            Self::IssueTransitions { .. } => "Loading the statuses to move to",
            Self::IssueEditMeta { .. } => "Loading the issue's choices",
            Self::TransitionIssue { .. } => "Saving the status",
            Self::SetPriority { .. } => "Saving the priority",
            Self::SetTitle { .. } => "Saving the title",
            Self::AssignIssue { .. } => "Saving the assignee",
            Self::AddComment { .. } => "Sending the comment",
            Self::AddRemoteLink { .. } => "Adding the web link",
        }
    }
}

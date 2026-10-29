//! The Jira source: every call the sync makes goes through the Jira client and its gateway (ADR 0025).

use std::collections::BTreeSet;
use std::future::Future;
use std::pin::Pin;

use super::source::{Answer, IssuePages, Source, SourceKind};
use crate::error::AppResult;
use crate::jira::dto::{BulkFetch, Comment, Field, Myself, Project, SearchPage, Status, Version};
use crate::jira::{JiraClient, SearchPages, SearchRequest};

pub struct JiraSource {
    client: JiraClient,
}

impl JiraSource {
    pub fn new(client: JiraClient) -> Self {
        Self { client }
    }
}

struct JiraPages<'a>(SearchPages<'a>);

impl IssuePages for JiraPages<'_> {
    fn next_page(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Option<AppResult<SearchPage>>> + Send + '_>> {
        Box::pin(self.0.next_page())
    }
}

impl Source for JiraSource {
    fn kind(&self) -> SourceKind {
        SourceKind::Jira
    }

    fn fields(&self) -> Answer<'_, Vec<Field>> {
        Box::pin(self.client.fields())
    }

    fn statuses(&self) -> Answer<'_, Vec<Status>> {
        Box::pin(self.client.statuses())
    }

    fn spaces(&self) -> Answer<'_, Vec<Project>> {
        Box::pin(self.client.projects())
    }

    fn versions<'a>(&'a self, space_key: &'a str) -> Answer<'a, Vec<Version>> {
        Box::pin(self.client.versions(space_key))
    }

    fn myself(&self) -> Answer<'_, Myself> {
        Box::pin(self.client.myself())
    }

    fn count<'a>(&'a self, query: &'a str) -> Answer<'a, u64> {
        Box::pin(self.client.approximate_count(query))
    }

    fn issues(&self, request: SearchRequest) -> Box<dyn IssuePages + '_> {
        Box::new(JiraPages(self.client.search_jql(request)))
    }

    fn fetch<'a>(&'a self, ids: &'a [String], fields: &'a [String]) -> Answer<'a, BulkFetch> {
        Box::pin(self.client.bulk_fetch(ids, fields))
    }

    fn comments<'a>(&'a self, issue_key: &'a str) -> Answer<'a, Vec<Comment>> {
        Box::pin(self.client.comments(issue_key))
    }

    fn used_custom_fields<'a>(&'a self, space_key: &'a str) -> Answer<'a, BTreeSet<String>> {
        Box::pin(self.client.used_custom_fields(space_key))
    }
}

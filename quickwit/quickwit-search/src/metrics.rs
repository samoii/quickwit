// Copyright (C) 2024 Quickwit, Inc.
//
// Quickwit is offered under the AGPL v3.0 and as commercial software.
// For commercial licensing, contact us at hello@quickwit.io.
//
// AGPL:
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

// See https://prometheus.io/docs/practices/naming/

use once_cell::sync::Lazy;
use quickwit_common::metrics::{
    exponential_buckets, new_counter, new_counter_vec, new_histogram, new_histogram_vec, Histogram,
    HistogramVec, IntCounter, IntCounterVec,
};

pub struct SearchMetrics {
    pub root_search_requests_total: IntCounterVec<1>,
    pub root_search_request_duration_seconds: HistogramVec<1>,
    pub leaf_search_requests_total: IntCounterVec<1>,
    pub leaf_search_request_duration_seconds: HistogramVec<1>,
    pub leaf_searches_splits_total: IntCounter,
    pub leaf_search_split_duration_secs: Histogram,
    pub job_assigned_total: IntCounterVec<1>,
}

impl Default for SearchMetrics {
    fn default() -> Self {
        SearchMetrics {
            root_search_requests_total: new_counter_vec(
                "root_search_requests_total",
                "Total number of root search gRPC requests processed.",
                "search",
                &[("kind", "server")],
                ["status"],
            ),
            root_search_request_duration_seconds: new_histogram_vec(
                "root_search_request_duration_seconds",
                "Duration of request in seconds.",
                "search",
                &[("kind", "server")],
                ["status"],
                exponential_buckets(0.001, 2.0, 15).unwrap(),
            ),
            leaf_search_requests_total: new_counter_vec(
                "leaf_search_requests_total",
                "Total number of gRPC requests processed.",
                "search",
                &[("kind", "server")],
                ["status"],
            ),
            leaf_search_request_duration_seconds: new_histogram_vec(
                "leaf_search_request_duration_seconds",
                "Duration of request in seconds.",
                "search",
                &[("kind", "server")],
                ["status"],
                exponential_buckets(0.001, 2.0, 15).unwrap(),
            ),
            leaf_searches_splits_total: new_counter(
                "leaf_searches_splits_total",
                "Number of leaf searches (count of splits) started.",
                "search",
                &[],
            ),
            leaf_search_split_duration_secs: new_histogram(
                "leaf_search_split_duration_secs",
                "Number of seconds required to run a leaf search over a single split. The timer \
                 starts after the semaphore is obtained.",
                "search",
                exponential_buckets(0.001, 2.0, 15).unwrap(),
            ),
            job_assigned_total: new_counter_vec(
                "job_assigned_total",
                "Number of job assigned to searchers, per affinity rank.",
                "search",
                &[],
                ["affinity"],
            ),
        }
    }
}

/// `SEARCH_METRICS` exposes a bunch a set of storage/cache related metrics through a prometheus
/// endpoint.
pub static SEARCH_METRICS: Lazy<SearchMetrics> = Lazy::new(SearchMetrics::default);

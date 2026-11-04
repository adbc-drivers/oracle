// -----------------------------------------------------------------------------
// Copyright (c) 2026 ADBC Drivers Contributors
//
// This software is dual-licensed to you under the Universal Permissive License
// (UPL) 1.0 as shown at https://oss.oracle.com/licenses/upl and Apache License
// 2.0 as shown at http://www.apache.org/licenses/LICENSE-2.0. You may choose
// either license.
//
// If you elect to accept the software under the Apache License, Version 2.0,
// the following applies:
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
// -----------------------------------------------------------------------------

//! Adapters between Oracle Arrow streams and ADBC readers.

use std::collections::VecDeque;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use arrow_array::{RecordBatch, RecordBatchReader};
use arrow_schema::{ArrowError, SchemaRef};

pub(crate) struct BatchReader {
    schema: SchemaRef,
    batches: VecDeque<RecordBatch>,
}

impl BatchReader {
    /// Creates a reader from at least one batch, retaining zero-row batches.
    pub(crate) fn new(batches: Vec<RecordBatch>) -> Self {
        let schema = batches
            .first()
            .expect("a batch is required to determine the reader schema")
            .schema();
        Self {
            schema,
            batches: batches.into(),
        }
    }
}

impl Iterator for BatchReader {
    type Item = std::result::Result<RecordBatch, ArrowError>;

    /// Returns the next batch or ends the stream when exhausted.
    fn next(&mut self) -> Option<Self::Item> {
        self.batches.pop_front().map(Ok)
    }
}

/// Preserves ADBC cancellation while returning native Arrow batches lazily.
pub(crate) struct CancellableQueryReader {
    batches: oracledb::ArrowBatchReader<'static>,
    cancelled: Arc<AtomicBool>,
}

impl CancellableQueryReader {
    /// Wraps a native stream with the statement's cancellation flag.
    pub(crate) fn new(
        batches: oracledb::ArrowBatchReader<'static>,
        cancelled: Arc<AtomicBool>,
    ) -> Self {
        Self { batches, cancelled }
    }
}

impl Iterator for CancellableQueryReader {
    type Item = std::result::Result<RecordBatch, ArrowError>;

    /// Returns the next native batch unless the statement was cancelled.
    fn next(&mut self) -> Option<Self::Item> {
        if self.cancelled.load(Ordering::Relaxed) {
            return None;
        }
        self.batches.next()
    }
}

impl RecordBatchReader for CancellableQueryReader {
    /// Returns the schema captured from the native query stream.
    fn schema(&self) -> SchemaRef {
        self.batches.schema()
    }
}

impl RecordBatchReader for BatchReader {
    /// Returns the schema captured from the first batch.
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}

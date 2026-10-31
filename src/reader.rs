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

//! Temporary adaptation of buffered Arrow batches to ADBC's reader interface.
// TODO: Replace this adapter when rust-oracledb provides a native RecordBatchReader.

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
    batch_size: usize,
    offset: usize,
    cancelled: Option<Arc<AtomicBool>>,
}

impl BatchReader {
    /// Creates a cancellable query reader that skips empty batches and limits
    /// batch rows.
    pub(crate) fn for_query(
        batches: Vec<RecordBatch>,
        batch_size: usize,
        cancelled: Arc<AtomicBool>,
    ) -> Self {
        let mut reader = Self::new(batches);
        reader.batches.retain(|batch| batch.num_rows() > 0);
        reader.batch_size = batch_size.max(1);
        reader.cancelled = Some(cancelled);
        reader
    }

    /// Creates a reader from at least one batch, retaining zero-row batches.
    pub(crate) fn new(batches: Vec<RecordBatch>) -> Self {
        let schema = batches
            .first()
            .expect("a batch is required to determine the reader schema")
            .schema();
        Self {
            schema,
            batches: batches.into(),
            batch_size: usize::MAX,
            offset: 0,
            cancelled: None,
        }
    }
}

impl Iterator for BatchReader {
    type Item = std::result::Result<RecordBatch, ArrowError>;

    /// Returns the next batch slice, or ends the stream on cancellation or
    /// exhaustion.
    fn next(&mut self) -> Option<Self::Item> {
        if self
            .cancelled
            .as_ref()
            .is_some_and(|cancelled| cancelled.load(Ordering::Relaxed))
        {
            self.batches.clear();
            return None;
        }
        let batch = self.batches.front()?;
        let row_count = (batch.num_rows() - self.offset).min(self.batch_size);
        let result = batch.slice(self.offset, row_count);
        self.offset += row_count;
        if self.offset == batch.num_rows() {
            self.batches.pop_front();
            self.offset = 0;
        }
        Some(Ok(result))
    }
}

impl RecordBatchReader for BatchReader {
    /// Returns the schema captured from the first batch.
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}

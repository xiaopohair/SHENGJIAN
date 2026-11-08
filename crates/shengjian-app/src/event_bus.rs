use crate::ports::EventSink;
use shengjian_contract::{CoreEvent, EventEnvelope, Generation};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Single ordered event stream shared by all services.
pub struct EventPublisher {
    next_seq: AtomicU64,
    sink: Arc<dyn EventSink>,
}

impl EventPublisher {
    pub fn new(sink: Arc<dyn EventSink>) -> Self {
        Self {
            next_seq: AtomicU64::new(0),
            sink,
        }
    }

    pub fn publish(&self, generation: Generation, event: CoreEvent) {
        let seq = self.next_seq.fetch_add(1, Ordering::SeqCst) + 1;
        self.sink.emit(EventEnvelope {
            seq,
            generation,
            event,
        });
    }

    pub fn last_seq(&self) -> u64 {
        self.next_seq.load(Ordering::SeqCst)
    }
}

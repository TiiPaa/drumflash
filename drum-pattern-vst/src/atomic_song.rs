use std::sync::Arc;

use crossbeam::queue::ArrayQueue;

use crate::pattern_bank::SongSequence;

/// Lock-free runtime controller for the song sequence.
///
/// The UI thread publishes full `SongSequence` snapshots to an SPSC queue.
/// The audio thread consumes the latest snapshot at the start of each process
/// block, avoiding any mutex on the hot path.
///
/// Persistence is still handled by `PatternBank` (`pattern-bank-v1`). This
/// controller is only a runtime cache so the audio thread can read the song
/// without locking `PatternBank`.
pub struct SongStateController {
    queue: ArrayQueue<SongSequence>,
}

impl SongStateController {
    /// Capacity of the internal SPSC queue. Four slots is more than enough for
    /// a 16-block song editor: even with very fast UI edits, the audio thread
    /// consumes at least once per process block.
    const QUEUE_CAPACITY: usize = 4;

    /// Create a new controller. The controller starts empty; the audio thread
    /// will sync its local copy from `PatternBank` in `initialize()`.
    pub fn new() -> Self {
        Self {
            queue: ArrayQueue::new(Self::QUEUE_CAPACITY),
        }
    }

    /// Publish a new song sequence from the UI thread.
    ///
    /// If the queue is full (audio thread has not consumed for several blocks),
    /// the oldest pending snapshots are dropped so the audio thread always sees
    /// the most recent state.
    pub fn publish(&self, song: SongSequence) {
        while self.queue.is_full() {
            let _ = self.queue.pop();
        }
        let _ = self.queue.push(song);
    }

    /// Consume the latest pending song sequence.
    ///
    /// Returns the most recent snapshot if any have been published since the
    /// last call. Older snapshots are drained and discarded.
    pub fn consume_latest(&self) -> Option<SongSequence> {
        let mut latest = None;
        while let Some(song) = self.queue.pop() {
            latest = Some(song);
        }
        latest
    }

    /// Number of snapshots currently in the queue (for diagnostics).
    #[allow(dead_code)]
    pub fn pending_count(&self) -> usize {
        self.queue.len()
    }
}

impl Default for SongStateController {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedSongStateController = Arc<SongStateController>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_and_consume_roundtrip() {
        let controller = SongStateController::new();
        let mut song = SongSequence::new();
        song.set_step(0, 2);
        song.set_repeat(0, 4);

        controller.publish(song.clone());
        let consumed = controller.consume_latest().expect("expected a snapshot");
        assert_eq!(consumed.slot_at(0), Some(2));
        assert_eq!(consumed.repeat_at(0), 4);
    }

    #[test]
    fn consume_latest_returns_most_recent() {
        let controller = SongStateController::new();
        for i in 1..=5u8 {
            let mut song = SongSequence::new();
            song.set_step(0, i as i8 - 1);
            controller.publish(song);
        }

        let latest = controller.consume_latest().expect("expected a snapshot");
        assert_eq!(latest.slot_at(0), Some(4));
    }

    #[test]
    fn drops_oldest_when_full() {
        let controller = SongStateController::new();
        for i in 0..(SongStateController::QUEUE_CAPACITY + 2) {
            let mut song = SongSequence::new();
            song.set_step(0, i as i8);
            controller.publish(song);
        }

        // Only the most recent snapshot should remain.
        let latest = controller.consume_latest().expect("expected a snapshot");
        let expected = (SongStateController::QUEUE_CAPACITY + 1) as i8;
        assert_eq!(latest.slot_at(0), Some(expected as usize));
    }
}

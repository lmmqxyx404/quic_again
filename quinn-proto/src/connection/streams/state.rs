use rustc_hash::FxHashMap;

use crate::{
    connection::{
        spaces::{Retransmits, ThinRetransmits},
        stats::FrameStats,
    },
    Dir, Side, StreamId, VarInt,
};

use super::{send::Send, StreamEvent};
use std::{collections::VecDeque, mem};

#[allow(unreachable_pub)] // fuzzing only
pub struct StreamsState {
    /// Whether the remote endpoint has opened any streams the application doesn't know about yet,
    /// per directionality
    opened: [bool; 2],
    /// Streams blocked on connection-level flow control or stream window space
    ///
    /// Streams are only added to this list when a write fails.
    pub(super) connection_blocked: Vec<StreamId>,

    // Set of streams that are currently open, or could be immediately opened by the peer
    pub(super) send: FxHashMap<StreamId, Option<Box<Send>>>,

    events: VecDeque<StreamEvent>,
    /// Connection-level flow control budget dictated by the peer
    pub(super) max_data: u64,
    /// Sum of current offsets of all send streams.
    pub(super) data_sent: u64,
    /// Total quantity of unacknowledged outgoing data
    pub(super) unacked_data: u64,
    /// Configured upper bound for `unacked_data`
    pub(super) send_window: u64,
}

impl StreamsState {
    #[allow(unreachable_pub)] // fuzzing only
    pub fn new(
        side: Side,
        max_remote_uni: VarInt,
        max_remote_bi: VarInt,
        send_window: u64,
        receive_window: VarInt,
        stream_receive_window: VarInt,
    ) -> Self {
        let mut this = Self {
            opened: [false, false],
            connection_blocked: Vec::new(),
            send: FxHashMap::default(),
            events: VecDeque::new(),
            max_data: 0,
            data_sent: 0,
            unacked_data: 0,
            send_window,
        };

        this
    }

    /// 2. Yield stream events
    pub(crate) fn poll(&mut self) -> Option<StreamEvent> {
        if let Some(dir) = Dir::iter().find(|&i| mem::replace(&mut self.opened[i as usize], false))
        {
            return Some(StreamEvent::Opened { dir });
        }
        if self.write_limit() > 0 {
            while let Some(id) = self.connection_blocked.pop() {
                let stream = match self.send.get_mut(&id).and_then(|s| s.as_mut()) {
                    None => continue,
                    Some(s) => s,
                };

                debug_assert!(stream.connection_blocked);
                stream.connection_blocked = false;

                // If it's no longer sensible to write to a stream (even to detect an error) then don't
                // report it.
                if stream.is_writable() && stream.max_data > stream.offset() {
                    return Some(StreamEvent::Writable { id });
                }
            }
        }

        self.events.pop_front()
    }

    /// Returns the maximum amount of data this is allowed to be written on the connection
    pub(crate) fn write_limit(&self) -> u64 {
        (self.max_data - self.data_sent).min(self.send_window - self.unacked_data)
    }

    pub(in crate::connection) fn write_control_frames(
        &mut self,
        buf: &mut Vec<u8>,
        pending: &mut Retransmits,
        retransmits: &mut ThinRetransmits,
        stats: &mut FrameStats,
        max_size: usize,
    ) {
        todo!()
    }
}
